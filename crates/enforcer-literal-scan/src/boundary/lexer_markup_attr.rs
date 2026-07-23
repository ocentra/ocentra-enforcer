//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_shared::read_quoted;
use crate::{LiteralCandidate, LiteralKind};
use enforcer_domain::scan_types::{LiteralSourceColumn, LiteralSourceLine};

pub(crate) fn consume_attribute_literal(
    line: &str,
    search: &str,
    eq: usize,
    offset: &mut usize,
) -> Option<(LiteralCandidate, usize)> {
    let after_equals = search.get(eq.checked_add(1)?..)?;
    let rest = after_equals.trim_start();
    let skipped = after_equals.len() - rest.len();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let (content, consumed) = read_quoted(rest, quote)?;
    let candidate = LiteralCandidate {
        text: content.into(),
        line: LiteralSourceLine::from_one_based(1),
        column: LiteralSourceColumn::from_one_based(*offset + eq + 1 + skipped + 1),
        kind: LiteralKind::Attribute,
        context: String::from(line).into(),
    };
    let advance = eq + 1 + skipped + consumed;
    Some((candidate, advance))
}
