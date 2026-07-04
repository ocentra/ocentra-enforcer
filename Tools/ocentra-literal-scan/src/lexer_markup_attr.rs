use crate::lexer_shared::read_quoted;
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn consume_attribute_literal(
    line: &str,
    search: &str,
    eq: usize,
    offset: &mut usize,
) -> Option<(LiteralCandidate, usize)> {
    let rest = &search[eq + 1..].trim_start();
    let skipped = search[eq + 1..].len() - rest.len();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let (content, consumed) = read_quoted(rest, quote)?;
    let candidate = LiteralCandidate {
        text: content,
        line: 0,
        column: *offset + eq + 1 + skipped + 1,
        kind: LiteralKind::Attribute,
        context: line.to_string(),
    };
    let advance = eq + 1 + skipped + consumed;
    Some((candidate, advance))
}
