use enforcer_harness::parsers::parse_diagnostics;

#[test]
fn text_adapters_keep_valid_diagnostics_and_skip_malformed_lines() {
    let text = "src/app.ts(10,5): error TS2322: Type mismatch\nFAILED tests/test_widget.py::test_draw - assertion failed\nnot a diagnostic";

    let diagnostics = parse_diagnostics("run-1", "tsc", text, "");

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].rule_id, "TS2322");
    assert_eq!(diagnostics[0].file, "src/app.ts");
    assert_eq!(diagnostics[0].line, 10);
    assert_eq!(diagnostics[1].rule_id, "pytest");
    assert_eq!(diagnostics[1].file, "tests/test_widget.py");
}
