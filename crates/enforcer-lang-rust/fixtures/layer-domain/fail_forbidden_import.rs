// FAIL fixture for RUST-LAYER-1.1: forbidden-crate import inside
// `src/domain/`. This fixture's file/RelPath (assigned by the validator's
// test, not the file's real location) is treated as living under
// `src/domain/`.
use reqwest::Client;

pub fn fetch(client: &Client) {
    let _ = client;
}
