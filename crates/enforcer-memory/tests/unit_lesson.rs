use enforcer_memory::lesson::parse_ledger;

#[test]
fn parses_header_and_separator_are_skipped() {
    let text = "\
# Ledger

| id | date | observed | lesson | landed-at | ships-via |
|---|---|---|---|---|---|
| L1 | 2026-07-04 | saw X | learned Y | commit abc | arc-16 |
| L2 | 2026-07-04 | saw Z | learned W | commit def | arc-05 |
";
    let rows = parse_ledger(text);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "L1");
    assert_eq!(rows[1].lesson, "learned W");
}

#[test]
fn ignores_prose_lines() {
    let text = "This is prose with a | pipe | in it but not a real row\n";
    let rows = parse_ledger(text);
    assert!(rows.is_empty());
}
