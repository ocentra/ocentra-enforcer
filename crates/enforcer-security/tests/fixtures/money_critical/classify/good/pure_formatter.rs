// PASS fixture for MONEY-CRIT-CLASSIFY.1: a pure formatter with zero
// value-touching signals. Must stay under CLASSIFY_THRESHOLD and be
// reported clean by the T2 classifier.

fn format_greeting(name: &str) -> String {
    format!("hello, {name}!")
}
