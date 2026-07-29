use enforcer_scan::boundary::coverage::decode_coverage_json;

#[test]
fn malformed_coverage_json_is_rejected_before_domain_projection() {
    let result = decode_coverage_json(r#"{"ranCount":"broken"}"#);
    assert!(
        result.is_err(),
        "malformed coverage should be rejected at boundary decode"
    );
    if let Err(error) = result {
        assert!(error.is_data() || error.is_syntax());
    }
}
