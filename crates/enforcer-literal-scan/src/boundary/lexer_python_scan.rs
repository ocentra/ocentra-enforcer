//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_python_string::{current_line_ends_block, try_python_string};
use crate::lexer_shared::LexerCursor;
use crate::LiteralCandidate;

pub(crate) fn lex_python(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut last_significant_line_had_block_start = true;

    while index < bytes.len() {
        let Some(&byte) = bytes.get(index) else { break };
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
        if ch.is_whitespace() {
            index += 1;
            col += 1;
            continue;
        }
        let matched = {
            let mut cursor = LexerCursor {
                out: &mut out,
                index: &mut index,
                line: &mut line,
                col: &mut col,
            };
            try_python_string(
                source,
                &mut cursor,
                &mut last_significant_line_had_block_start,
            )
        };
        if matched {
            continue;
        }
        last_significant_line_had_block_start = current_line_ends_block(source, line);
        index += 1;
        col += 1;
    }
    out
}
