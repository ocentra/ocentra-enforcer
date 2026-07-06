use enforcer_memory::search::{DocumentKind, SearchDocument};

#[test]
fn label_boost_orders_function_above_route_above_type() {
    assert!(DocumentKind::Function.label_boost() > DocumentKind::Route.label_boost());
    assert!(DocumentKind::Route.label_boost() > DocumentKind::Type.label_boost());
}

#[test]
fn snippet_truncates_long_text_at_char_boundary() {
    let long = "a".repeat(500);
    let doc = SearchDocument::new("id1", DocumentKind::File, long);
    assert!(doc.snippet.ends_with("..."));
    assert!(doc.snippet.len() <= 244);
}

#[test]
fn snippet_leaves_short_text_untouched() {
    let doc = SearchDocument::new("id1", DocumentKind::File, "short text");
    assert_eq!(doc.snippet, "short text");
}
