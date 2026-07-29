// X06.9 fixture: a second Rust file so Symbol/CodeGraph runners have a
// caller/callee pair to traverse (widget.rs calls into lib.rs's
// parse_config_file).

use crate::lib::parse_config_file;

pub fn load_widget_settings(path: &str) -> Settings {
    let config = parse_config_file(path);
    Settings { source: config.path }
}

pub struct Settings {
    pub source: String,
}
