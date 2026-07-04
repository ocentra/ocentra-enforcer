//! Seeded-violation fixture: `unwrap()` in first-party non-test code trips
//! `T1-RUSTERR.1` (enforcer-lang-rust's error_handling validator), routed
//! here through the Rust family.
pub fn risky(value: Option<i32>) -> i32 {
    value.unwrap()
}
