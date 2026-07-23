//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn lex_shell(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while index < bytes.len() {
        let Some(&byte) = bytes.get(index) else {
            break;
        };
        let ch = byte as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while bytes.get(index).is_some_and(|byte| *byte as char != '\n') {
                index += 1;
                col += 1;
            }
            continue;
        }
        if let Some((literal, consumed)) = read_shell_literal(source, index, line, col) {
            out.push(literal);
            let Some(next_index) = index.checked_add(consumed) else {
                break;
            };
            let Some(consumed_text) = source.get(index..next_index) else {
                break;
            };
            advance_position(consumed_text, &mut line, &mut col);
            index = next_index;
            continue;
        }
        index += 1;
        col += 1;
    }
    out
}

fn read_shell_literal(
    source: &str,
    index: usize,
    line: usize,
    col: usize,
) -> Option<(LiteralCandidate, usize)> {
    let ch = source.as_bytes().get(index).map(|byte| *byte as char)?;
    if !matches!(ch, '"' | '\'' | '`') {
        return None;
    }
    let rest = source.get(index..)?;
    let (content, consumed) = read_quoted(rest, ch)?;
    let kind = if ch == '`' {
        LiteralKind::Template
    } else {
        LiteralKind::Normal
    };
    Some((
        candidate(&content, line, col, kind, line_at(source, line)),
        consumed,
    ))
}
