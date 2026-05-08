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
    let mut missing_terminal_count = 0usize;
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
        missing_terminal_count += count_missing_terminal_keys(&payload, &segments);
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

    if missing_terminal_count > 0 {
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

fn count_missing_terminal_keys(value: &Value, segments: &[PathSegment]) -> usize {
    let Some((PathSegment::Key(terminal_key), parent_segments)) = segments.split_last() else {
        return 0;
    };
    let mut parents = Vec::new();

    collect_values_at_path(value, parent_segments, &mut parents);

    parents
        .iter()
        .filter(|parent| {
            parent
                .as_object()
                .map(|object| !object.contains_key(terminal_key))
                .unwrap_or(false)
        })
        .count()
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
