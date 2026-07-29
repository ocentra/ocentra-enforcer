// FAIL fixture for T1-RUSTERR.1: `.unwrap()` in first-party
// (non-#[cfg(test)]) code.
fn parse_it(raw: &str) -> i32 {
    raw.parse::<i32>().unwrap()
}
