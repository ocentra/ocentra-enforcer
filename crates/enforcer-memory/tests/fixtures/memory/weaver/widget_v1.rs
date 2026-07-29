// Fixture: the "before" revision of a small source file, used by
// tests/weaver_enrichment.rs to derive a real content hash for a
// `WeaverEvent::NodeChanged`/`FileChanged` pair rather than an inline
// synthetic string -- mirrors the code_graph indexer fixtures' policy
// of exercising real file content instead of ad hoc literals.

pub struct Widget {
    pub name: String,
}

impl Widget {
    pub fn render(&self) -> String {
        format!("<widget>{}</widget>", self.name)
    }
}
