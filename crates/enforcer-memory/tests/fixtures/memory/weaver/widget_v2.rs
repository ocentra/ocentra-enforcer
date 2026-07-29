// Fixture: the "after" revision of widget_v1.rs -- same symbol,
// changed body, so its content hash differs. Used by
// tests/weaver_enrichment.rs to prove a `FileChanged` event carrying
// this hash invalidates the cached summary for the file.

pub struct Widget {
    pub name: String,
}

impl Widget {
    pub fn render(&self) -> String {
        format!("<widget class=\"v2\">{}</widget>", self.name)
    }
}
