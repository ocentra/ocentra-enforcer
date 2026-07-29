// Minimal pre-publish smoke fixture: a clean file with no seeded
// violation. A release binary run against this fixture must exit
// Success (0) -- see release_pipeline::gate_release.
fn main() {
    println!("smoke-pass");
}
