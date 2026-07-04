use crate::lexer_python_string::{current_line_ends_block, try_python_string};
use crate::LiteralCandidate;

pub(crate) fn lex_python(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut last_significant_line_had_block_start = true;

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
        if ch.is_whitespace() {
            index += 1;
            col += 1;
            continue;
        }
        if try_python_string(
            source,
            &mut out,
            &mut index,
            &mut line,
            &mut col,
            &mut last_significant_line_had_block_start,
        ) {
            continue;
        }
        last_significant_line_had_block_start = current_line_ends_block(source, line);
        index += 1;
        col += 1;
    }
    out
}
