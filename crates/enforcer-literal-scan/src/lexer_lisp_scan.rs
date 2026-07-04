use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn lex_lisp(source: &str) -> Vec<LiteralCandidate> {
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
        if ch == ';' {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '"' {
            if let Some((content, consumed)) = read_quoted(&source[index..], '"') {
                out.push(candidate(&content, line, col, LiteralKind::Normal, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        index += 1;
        col += 1;
    }
    out
}
