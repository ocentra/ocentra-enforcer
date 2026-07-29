// Minimal pre-publish smoke fixture: a seeded T1 violation (an
// `unwrap()` call, banned under this repo's own rules). A release
// binary run against this fixture must exit Violations (1) -- never a
// panic, never a silent Success. See release_pipeline::gate_release.
fn main() {
    let maybe: Option<i32> = None;
    let value = maybe.unwrap();
    println!("smoke-fail: {value}");
}
