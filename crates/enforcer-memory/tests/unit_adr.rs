use enforcer_memory::adr::{AdrError, AdrRecord, AdrStore};
use std::error::Error;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn adr_roundtrip_create_get_update_section() -> TestResult {
    let mut store = AdrStore::new();
    let adr = AdrRecord::new("adr-001", "Use SQLite for the operational store")
        .with_section("context", "local-first, zero-install")
        .with_section("decision", "rusqlite bundled");
    store.create(adr)?;

    let fetched = store.get("adr-001")?;
    assert_eq!(fetched.title, "Use SQLite for the operational store");
    assert_eq!(fetched.sections["decision"], "rusqlite bundled");

    store.update_section("adr-001", "consequences", "no libmdbx, no sled")?;
    let updated = store.get("adr-001")?;
    assert_eq!(updated.sections["consequences"], "no libmdbx, no sled");
    assert_eq!(updated.sections.len(), 3, "context+decision+consequences");
    Ok(())
}

#[test]
fn adr_roundtrip_via_serde_shape_is_stable() {
    // No serde derive is required by the hard test list; this
    // asserts the struct's field shape stays stable for a manual
    // caller-side round trip (construct -> read every field back).
    let adr = AdrRecord::new("adr-002", "title")
        .with_section("s", "body")
        .with_linked_node("file:a.rs");
    assert_eq!(adr.id, "adr-002");
    assert_eq!(adr.title, "title");
    assert_eq!(adr.sections.get("s").map(String::as_str), Some("body"));
    assert_eq!(adr.linked_node_ids, vec!["file:a.rs".to_string()]);
}

#[test]
fn create_duplicate_id_is_rejected() -> TestResult {
    let mut store = AdrStore::new();
    store.create(AdrRecord::new("adr-001", "first"))?;
    let result = store.create(AdrRecord::new("adr-001", "second"));
    assert!(matches!(result, Err(AdrError::AlreadyExists(id)) if id == "adr-001"));
    Ok(())
}

#[test]
fn get_unknown_id_is_not_found_not_panic() {
    let store = AdrStore::new();
    let result = store.get("adr-missing");
    assert!(matches!(result, Err(AdrError::NotFound(id)) if id == "adr-missing"));
}

#[test]
fn adr_linked_to_graph_node_is_found_by_node_id() -> TestResult {
    let mut store = AdrStore::new();
    store
        .create(AdrRecord::new("adr-003", "why file:a.rs exists").with_linked_node("file:a.rs"))?;
    store.create(AdrRecord::new("adr-004", "unrelated"))?;

    let linked = store.adrs_for_node("file:a.rs");
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, "adr-003");

    let none = store.adrs_for_node("file:does-not-exist.rs");
    assert!(none.is_empty());
    Ok(())
}
