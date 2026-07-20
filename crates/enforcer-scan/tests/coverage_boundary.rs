use enforcer_scan::boundary::coverage::decode_coverage_json;

#[test]
fn malformed_coverage_json_is_rejected_before_domain_projection() {
    let error = decode_coverage_json(r#"{"ranCount":"broken"}"#)
        .expect_err("malformed coverage should be rejected at boundary decode");
    assert!(error.is_data() || error.is_syntax());
}
