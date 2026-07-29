use std::fs;

struct Widget;

fn list_widgets() -> Vec<Widget> {
    load_from_disk();
    Vec::new()
}

fn load_from_disk() {
    fs::read_to_string("widgets.json").ok();
    validate();
}

fn validate() {}

#[test]
fn list_widgets_returns_empty_by_default() {
    assert!(list_widgets().is_empty());
}
