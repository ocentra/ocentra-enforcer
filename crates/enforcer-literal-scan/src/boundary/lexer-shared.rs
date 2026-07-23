//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use super::{LiteralCandidate, LiteralKind};
use enforcer_domain::scan_types::{LiteralSourceColumn, LiteralSourceLine};

pub(crate) struct LexerCursor<'a> {
    pub(crate) out: &'a mut Vec<LiteralCandidate>,
    pub(crate) index: &'a mut usize,
    pub(crate) line: &'a mut usize,
    pub(crate) col: &'a mut usize,
}
pub(crate) fn read_quoted(source: &str, quote: char) -> Option<(String, usize)> {
    let mut chars = source.char_indices();
    let (_, first) = chars.next()?;
    if first != quote {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for (idx, ch) in chars {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            out.push(ch);
            continue;
        }
        if ch == quote {
            return Some((out, idx + ch.len_utf8()));
        }
        out.push(ch);
    }
    None
}

pub(crate) fn advance_position(text: &str, line: &mut usize, col: &mut usize) {
    for ch in text.chars() {
        if ch == '\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
    }
}

pub(crate) fn candidate(
    text: &str,
    line: usize,
    column: usize,
    kind: LiteralKind,
    context: Option<String>,
) -> LiteralCandidate {
    LiteralCandidate {
        text: String::from(text).into(),
        line: LiteralSourceLine::from_one_based(line),
        column: LiteralSourceColumn::from_one_based(column),
        kind,
        context: context.unwrap_or_default().into(),
    }
}

pub(crate) fn line_at(source: &str, line: usize) -> Option<String> {
    source.lines().nth(line.saturating_sub(1)).map(String::from)
}
