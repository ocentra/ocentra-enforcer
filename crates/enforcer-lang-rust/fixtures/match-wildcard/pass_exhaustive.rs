// PASS fixture for RUST-MATCH-NO-WILDCARD: exhaustive per-variant arms, no
// catch-all `_ =>`.
enum Status {
    Active,
    Inactive,
    Pending,
}

fn describe(s: Status) -> &'static str {
    match s {
        Status::Active => "active",
        Status::Inactive => "inactive",
        Status::Pending => "pending",
    }
}
