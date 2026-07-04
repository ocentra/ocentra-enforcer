// Seeded T1 violation fixture (workpack c04): a first-party `.unwrap()`
// fires `T1-RUSTERR.1` (Severity::Error) via
// `enforcer_lang_rust::rules::error_handling::ErrorHandlingValidator`.
fn parse_it(raw: &str) -> i32 {
    raw.parse::<i32>().unwrap()
}
