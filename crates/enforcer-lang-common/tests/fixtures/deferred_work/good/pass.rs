// Fixture: a correctly annotated deferred-work stub. Must NOT trip
// `DEFER-1.1` — the DEFERRED annotation is the only valid escape hatch.
fn compute() -> i32 {
    // TODO: implement the real calculation DEFERRED(#ARC-42)[revisit:2027-01-01]
    0
}
