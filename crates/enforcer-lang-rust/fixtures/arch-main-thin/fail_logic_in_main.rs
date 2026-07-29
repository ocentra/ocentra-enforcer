// FAIL fixture for RUST-ARCH-1.1: business logic function defined directly
// in `main.rs` (this fixture is validated AS a main.rs by the test).
fn compute_total(items: &[i32]) -> i32 {
    items.iter().sum()
}

fn main() {
    let items = [1, 2, 3];
    println!("{}", compute_total(&items));
}
