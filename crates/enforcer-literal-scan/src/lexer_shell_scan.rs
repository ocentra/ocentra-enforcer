use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn lex_shell(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if let Some((literal, consumed)) = parse_shell_string(source, index, line, col) {
            out.push(literal);
            advance_position(&source[index..index + consumed], &mut line, &mut col);
            index += consumed;
            continue;
        }
        index += 1;
        col += 1;
    }
    out
}

fn parse_shell_string(
    source: &str,
    index: usize,
    line: usize,
    col: usize,
) -> Option<(LiteralCandidate, usize)> {
    let ch = source.as_bytes().get(index).map(|byte| *byte as char)?;
    if !matches!(ch, '"' | '\'' | '`') {
        return None;
    }
    let (content, consumed) = read_quoted(&source[index..], ch)?;
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
