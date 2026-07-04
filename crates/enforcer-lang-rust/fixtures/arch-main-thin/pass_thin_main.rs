// PASS fixture for RUST-ARCH-1.1: `main.rs` only parses args and calls
// `run()`; no business logic defined here.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    my_crate::run(&args);
}
