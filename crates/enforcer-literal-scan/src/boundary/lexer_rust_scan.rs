use crate::lexer_rust_string::{
    try_rust_byte_string, try_rust_char_or_lifetime, try_rust_raw_string, try_rust_standard_string,
    update_block_comment_state,
};
use crate::lexer_shared::LexerCursor;
use crate::LiteralCandidate;

pub(crate) fn lex_rust(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut block_depth = 0usize;

    while index < bytes.len() {
        let Some(&byte) = bytes.get(index) else { break };
        let ch = byte as char;
        let next = bytes.get(index + 1).map(|b| *b as char);
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if block_depth > 0 {
            update_block_comment_state(ch, next, &mut block_depth, &mut index, &mut col);
            continue;
        }
        if ch == '/' && next == Some('/') {
            while bytes.get(index).is_some_and(|byte| *byte as char != '\n') {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('*') {
            block_depth = 1;
            index += 2;
            col += 2;
            continue;
        }
        if try_rust_raw_string(source, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        let matched = {
            let mut cursor = LexerCursor {
                out: &mut out,
                index: &mut index,
                line: &mut line,
                col: &mut col,
            };
            try_rust_byte_string(source, ch, next, &mut cursor)
                || try_rust_standard_string(source, ch, &mut cursor)
        };
        if matched {
            continue;
        }
        if try_rust_char_or_lifetime(source, ch, &mut index, &mut col, &mut line) {
            continue;
        }
        index += 1;
        col += 1;
    }
    out
}
