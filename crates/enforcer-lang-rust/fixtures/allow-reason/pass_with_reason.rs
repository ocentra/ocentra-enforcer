// PASS fixture for RUST-ALLOW-1.1: `#[allow(...)]` carries `reason = "..."`.
#[allow(dead_code, reason = "kept for the upcoming plugin API, tracked in #1234")]
fn unused_helper() {}
