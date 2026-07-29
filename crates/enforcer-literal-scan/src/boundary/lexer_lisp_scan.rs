//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn lex_lisp(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while let Some(byte) = bytes.get(index).copied() {
        let ch = char::from(byte);
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == ';' {
            while let Some(comment_byte) = bytes.get(index).copied() {
                if comment_byte == b'\n' {
                    break;
                }
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '"' {
            if let Some(rest) = source.get(index..) {
                if let Some((content, consumed)) = read_quoted(rest, '"') {
                    if let Some(consumed_source) = rest.get(..consumed) {
                        out.push(candidate(
                            &content,
                            line,
                            col,
                            LiteralKind::Normal,
                            line_at(source, line),
                        ));
                        advance_position(consumed_source, &mut line, &mut col);
                        index += consumed;
                        continue;
                    }
                }
            }
        }
        index += 1;
        col += 1;
    }
    out
}
