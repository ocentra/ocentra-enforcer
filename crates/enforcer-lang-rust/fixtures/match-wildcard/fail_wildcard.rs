// FAIL fixture for RUST-MATCH-NO-WILDCARD: catch-all `_ =>` arm on a match
// over an internal enum, instead of exhaustive per-variant arms.
enum Status {
    Active,
    Inactive,
    Pending,
}

fn describe(s: Status) -> &'static str {
    match s {
        Status::Active => "active",
        _ => "other",
    }
}
