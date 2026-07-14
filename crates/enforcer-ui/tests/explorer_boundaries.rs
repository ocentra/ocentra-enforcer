use enforcer_ui::explorer::split_skill_forms;

#[test]
fn explorer_preserves_unterminated_dense_block_as_verbose_text() {
    let raw = "# Skill\n<!-- ai-dense -->";

    let (dense, verbose) = split_skill_forms(raw);

    assert!(dense.is_empty());
    assert_eq!(verbose, raw);
}

#[test]
fn explorer_splits_dense_block_at_document_boundaries() {
    let raw = "<!-- ai-dense -->\nkey: value\n<!-- /ai-dense -->";

    let (dense, verbose) = split_skill_forms(raw);

    assert_eq!(dense, "key: value");
    assert!(verbose.is_empty());
}
