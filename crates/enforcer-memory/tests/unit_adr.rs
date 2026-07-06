use enforcer_memory::adr::{AdrDocument, AdrError, AdrRecord, AdrStore};
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

// ---------------------------------------------------------------------
// Baseline whole-document API (refs/x06-baseline-tool-schemas.md §14):
// get_document/update_document/list_document_headings -- a separate
// address space from the section-record API above, keyed by a
// project-ish id rather than an ADR id.
// ---------------------------------------------------------------------

#[test]
fn get_document_on_never_stored_project_is_no_adr_with_empty_content() {
    let store = AdrStore::new();
    let doc = store.get_document("proj-a");
    assert_eq!(
        doc,
        AdrDocument {
            content: String::new(),
            no_adr: true,
        }
    );
}

#[test]
fn update_document_then_get_document_roundtrips_whole_markdown() {
    let mut store = AdrStore::new();
    let markdown = "## PURPOSE\nlocal-first\n\n## STACK\nrust\n";
    let previous = store.update_document("proj-a", markdown);
    assert_eq!(previous, None, "no prior document existed");

    let doc = store.get_document("proj-a");
    assert_eq!(doc.content, markdown);
    assert!(!doc.no_adr);
}

#[test]
fn update_document_overwrite_is_wholesale_not_a_merge() {
    let mut store = AdrStore::new();
    store.update_document("proj-a", "## PURPOSE\nfirst draft\n");
    let previous = store.update_document("proj-a", "## PURPOSE\nsecond draft\n");
    assert_eq!(previous, Some("## PURPOSE\nfirst draft\n".to_string()));

    let doc = store.get_document("proj-a");
    assert_eq!(doc.content, "## PURPOSE\nsecond draft\n");
    assert!(
        !doc.content.contains("first draft"),
        "update must replace wholesale, not append/merge"
    );
}

#[test]
fn list_document_headings_returns_heading_lines_verbatim() {
    let mut store = AdrStore::new();
    store.update_document(
        "proj-a",
        "intro text\n## PURPOSE\nbody\n### Sub-point\n## STACK\nmore body\n",
    );
    let headings = store.list_document_headings("proj-a");
    assert_eq!(headings, vec!["## PURPOSE", "### Sub-point", "## STACK"]);
}

#[test]
fn list_document_headings_on_unstored_project_is_empty_not_an_error() {
    let store = AdrStore::new();
    assert!(store.list_document_headings("proj-missing").is_empty());
}

#[test]
fn document_and_section_record_address_spaces_are_independent() -> TestResult {
    let mut store = AdrStore::new();
    store.create(AdrRecord::new("adr-001", "unrelated section record"))?;
    store.update_document("adr-001", "## PURPOSE\nsame id, different address space\n");

    // The section-record API and the whole-document API share no state
    // even when given the same id/project string.
    let record = store.get("adr-001")?;
    assert_eq!(record.title, "unrelated section record");
    let doc = store.get_document("adr-001");
    assert_eq!(
        doc.content,
        "## PURPOSE\nsame id, different address space\n"
    );
    Ok(())
}
