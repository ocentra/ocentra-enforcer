use crate::lexer_c_like_string::{
    try_standard_string, try_template_string, try_triple_string, try_verbatim_string,
};
use crate::{LanguageSpec, LiteralCandidate};

pub(crate) fn lex_c_like(
    source: &str,
    language: LanguageSpec,
    _rel: &str,
    ts_mode: bool,
) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut block_comment = false;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        let next = bytes.get(index + 1).map(|b| *b as char);
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if block_comment {
            if ch == '*' && next == Some('/') {
                block_comment = false;
                index += 2;
                col += 2;
            } else {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('*') {
            block_comment = true;
            index += 2;
            col += 2;
            continue;
        }
        if try_triple_string(source, language, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        if try_standard_string(source, language, ts_mode, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        if try_template_string(source, language, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        if try_verbatim_string(source, ch, next, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        index += 1;
        col += 1;
    }
    out
}
