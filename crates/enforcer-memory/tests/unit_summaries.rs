use enforcer_memory::summaries::SummaryStore;

#[test]
fn invalidate_marks_existing_summary_stale_without_deleting_it() {
    let mut store = SummaryStore::new();
    store.set_summary("src/lib.rs", "a summary");
    assert!(!store.is_stale("src/lib.rs").is_stale());

    store.invalidate("src/lib.rs");

    assert!(store.is_stale("src/lib.rs").is_stale());
    assert_eq!(
        store.get("src/lib.rs").map(|e| e.text.as_str()),
        Some("a summary")
    );
}

#[test]
fn missing_summary_is_considered_stale() {
    let store = SummaryStore::new();
    assert!(store.is_stale("never/seen.rs").is_stale());
}

#[test]
fn remove_deletes_the_entry_entirely() {
    let mut store = SummaryStore::new();
    store.set_summary("src/lib.rs", "a summary");
    store.remove("src/lib.rs");
    assert!(store.get("src/lib.rs").is_none());
}

#[test]
fn deleting_a_file_unlinks_its_entities() {
    let mut store = SummaryStore::new();
    store.link_entity("sym:src/lib.rs:1:foo");
    store.associate_entity_with_path("sym:src/lib.rs:1:foo", "src/lib.rs");
    assert!(store.is_entity_linked("sym:src/lib.rs:1:foo").is_linked());

    store.unlink_entities_for_path("src/lib.rs");

    assert!(!store.is_entity_linked("sym:src/lib.rs:1:foo").is_linked());
}
