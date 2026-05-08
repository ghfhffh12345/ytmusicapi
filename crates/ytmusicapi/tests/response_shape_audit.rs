#[path = "support/response_shape.rs"]
mod response_shape;

use response_shape::{audit_fixture_paths, load_expectations, observed_status};

const AUDIT_RAW_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/audit/raw");
const TEMP_CAPTURE_MODULE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/capture.rs");
const TEMP_CAPTURE_EXAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/capture_raw_fixtures.rs"
);
const EXPECTED_STATUS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/audit/expected_shape_status.json"
);

#[test]
fn temporary_capture_plumbing_has_been_removed() {
    assert!(
        !std::path::Path::new(TEMP_CAPTURE_MODULE).exists(),
        "temporary capture module should not exist at {TEMP_CAPTURE_MODULE}"
    );
    assert!(
        !std::path::Path::new(TEMP_CAPTURE_EXAMPLE).exists(),
        "temporary capture example should not exist at {TEMP_CAPTURE_EXAMPLE}"
    );
}

#[test]
fn expected_shape_status_file_uses_known_status_variants() {
    let expectations = load_expectations(EXPECTED_STATUS_FILE).unwrap();

    assert!(
        !expectations.is_empty(),
        "expected at least one committed response-shape expectation"
    );
}

#[test]
fn committed_audit_fixtures_match_expected_shape_statuses() {
    let fixtures = audit_fixture_paths(AUDIT_RAW_DIR).unwrap();
    let expectations = load_expectations(EXPECTED_STATUS_FILE).unwrap();

    assert!(
        !fixtures.is_empty(),
        "expected committed audit fixtures under {AUDIT_RAW_DIR}"
    );

    for expectation in expectations {
        let observed = observed_status(AUDIT_RAW_DIR, &fixtures, &expectation).unwrap();

        assert_eq!(
            observed, expectation.status,
            "{} at {} over {:?}",
            expectation.description, expectation.path, expectation.fixtures
        );
    }
}
