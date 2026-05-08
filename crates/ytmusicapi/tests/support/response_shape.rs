use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use walkdir::WalkDir;

#[derive(Clone, Debug, Deserialize)]
pub struct AuditExpectation {
    pub description: String,
    pub fixtures: Vec<String>,
    pub path: String,
    pub status: ShapeStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShapeStatus {
    AlwaysPresent,
    VariantOnly,
    ItemOptional,
    TypeUnstable,
}

#[derive(Debug)]
enum PathSegment {
    Key(String),
    ArrayItems,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum JsonKind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

pub fn audit_fixture_paths(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let root = root.as_ref();
    let mut paths = Vec::new();

    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry?;

        if entry.file_type().is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
        {
            paths.push(entry.path().strip_prefix(root)?.to_path_buf());
        }
    }

    Ok(paths)
}

pub fn load_expectations(path: impl AsRef<Path>) -> Result<Vec<AuditExpectation>, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let expectations = serde_json::from_str(&contents)?;

    Ok(expectations)
}

pub fn observed_status(
    root: impl AsRef<Path>,
    fixture_paths: &[PathBuf],
    expectation: &AuditExpectation,
) -> Result<ShapeStatus, Box<dyn Error>> {
    if expectation.fixtures.is_empty() {
        return Err(format!("{} does not select any fixtures", expectation.description).into());
    }

    let root = root.as_ref();
    let available = fixture_paths
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect::<HashSet<_>>();
    let segments = parse_path(&expectation.path)?;
    let mut fixture_count = 0usize;
    let mut fixtures_with_values = 0usize;
    let mut value_count = 0usize;
    let mut missing_path_count = 0usize;
    let mut observed_kinds = HashSet::new();

    for fixture in &expectation.fixtures {
        validate_fixture_reference(fixture)?;

        if !available.contains(fixture) {
            return Err(format!(
                "{} references missing audit fixture {fixture}",
                expectation.description
            )
            .into());
        }

        let payload: Value = serde_json::from_str(&fs::read_to_string(root.join(fixture))?)?;
        let mut values = Vec::new();
        collect_values_at_path(&payload, &segments, &mut values);

        if !values.is_empty() {
            fixtures_with_values += 1;
        }

        value_count += values.len();
        missing_path_count += count_missing_path_segments_under_arrays(&payload, &segments);
        observed_kinds.extend(values.into_iter().map(json_kind));
        fixture_count += 1;
    }

    if value_count == 0 {
        return Err(format!(
            "{} path {} was absent from all selected fixtures",
            expectation.description, expectation.path
        )
        .into());
    }

    if observed_kinds.len() > 1 {
        return Ok(ShapeStatus::TypeUnstable);
    }

    if missing_path_count > 0 {
        return Ok(ShapeStatus::ItemOptional);
    }

    if fixtures_with_values == fixture_count {
        return Ok(ShapeStatus::AlwaysPresent);
    }

    Ok(ShapeStatus::VariantOnly)
}

fn validate_fixture_reference(fixture: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(fixture);

    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!("audit fixture reference must stay under raw/: {fixture}").into());
    }

    Ok(())
}

fn parse_path(path: &str) -> Result<Vec<PathSegment>, Box<dyn Error>> {
    let path = path.strip_prefix("$.").unwrap_or(path);
    let path = path.strip_prefix('$').unwrap_or(path);

    if path.is_empty() {
        return Err("response shape path cannot be empty".into());
    }

    let mut segments = Vec::new();

    for part in path.split('.') {
        if part.is_empty() {
            return Err(format!("response shape path has an empty segment: {path}").into());
        }

        if let Some(key) = part.strip_suffix("[]") {
            if key.is_empty() {
                segments.push(PathSegment::ArrayItems);
            } else {
                segments.push(PathSegment::Key(key.to_owned()));
                segments.push(PathSegment::ArrayItems);
            }
        } else {
            segments.push(PathSegment::Key(part.to_owned()));
        }
    }

    Ok(segments)
}

fn collect_values_at_path<'a>(
    value: &'a Value,
    segments: &[PathSegment],
    values: &mut Vec<&'a Value>,
) {
    match segments.split_first() {
        None => values.push(value),
        Some((PathSegment::Key(key), rest)) => {
            if let Value::Object(object) = value {
                if let Some(next) = object.get(key) {
                    collect_values_at_path(next, rest, values);
                }
            }
        }
        Some((PathSegment::ArrayItems, rest)) => {
            if let Value::Array(items) = value {
                for item in items {
                    collect_values_at_path(item, rest, values);
                }
            }
        }
    }
}

fn count_missing_path_segments_under_arrays(value: &Value, segments: &[PathSegment]) -> usize {
    count_missing_path_segments_under_arrays_inner(value, segments, false)
}

fn count_missing_path_segments_under_arrays_inner(
    value: &Value,
    segments: &[PathSegment],
    under_array_item: bool,
) -> usize {
    match segments.split_first() {
        None => 0,
        Some((PathSegment::Key(key), rest)) => match value {
            Value::Object(object) => match object.get(key) {
                Some(next) => {
                    count_missing_path_segments_under_arrays_inner(next, rest, under_array_item)
                }
                None if under_array_item => 1,
                None => 0,
            },
            _ => 0,
        },
        Some((PathSegment::ArrayItems, rest)) => match value {
            Value::Array(items) => items
                .iter()
                .map(|item| count_missing_path_segments_under_arrays_inner(item, rest, true))
                .sum(),
            _ => 0,
        },
    }
}

fn json_kind(value: &Value) -> JsonKind {
    match value {
        Value::Null => JsonKind::Null,
        Value::Bool(_) => JsonKind::Bool,
        Value::Number(_) => JsonKind::Number,
        Value::String(_) => JsonKind::String,
        Value::Array(_) => JsonKind::Array,
        Value::Object(_) => JsonKind::Object,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn classifies_missing_intermediate_keys_under_arrays_as_item_optional() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("fixture.json"),
            r#"{
  "items": [
    { "renderer": { "title": "Present" } },
    { "otherRenderer": { "title": "Different item shape" } }
  ]
}"#,
        )
        .unwrap();
        let fixtures = audit_fixture_paths(dir.path()).unwrap();
        let expectation = AuditExpectation {
            description: "sibling item lacks the intermediate renderer key".to_owned(),
            fixtures: vec!["fixture.json".to_owned()],
            path: "items[].renderer.title".to_owned(),
            status: ShapeStatus::ItemOptional,
        };

        let observed = observed_status(dir.path(), &fixtures, &expectation).unwrap();

        assert_eq!(observed, ShapeStatus::ItemOptional);
    }

    #[test]
    fn classifies_mixed_value_kinds_as_type_unstable() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("string.json"),
            r#"{ "item": { "id": "abc" } }"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("number.json"),
            r#"{ "item": { "id": 123 } }"#,
        )
        .unwrap();
        let fixtures = audit_fixture_paths(dir.path()).unwrap();
        let expectation = AuditExpectation {
            description: "same path changes JSON value kind".to_owned(),
            fixtures: vec!["number.json".to_owned(), "string.json".to_owned()],
            path: "item.id".to_owned(),
            status: ShapeStatus::TypeUnstable,
        };

        let observed = observed_status(dir.path(), &fixtures, &expectation).unwrap();

        assert_eq!(observed, ShapeStatus::TypeUnstable);
    }
}
