//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_c_like_string::{
    try_standard_string, try_template_string, try_triple_string, try_verbatim_string,
};
use crate::lexer_shared::LexerCursor;
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
        let Some(&byte) = bytes.get(index) else {
            break;
        };
        let ch = byte as char;
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
            while bytes.get(index).is_some_and(|byte| *byte as char != '\n') {
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
        let matched = {
            let mut cursor = LexerCursor {
                out: &mut out,
                index: &mut index,
                line: &mut line,
                col: &mut col,
            };
            try_triple_string(source, language, &mut cursor)
                || try_standard_string(source, language, ts_mode, &mut cursor)
                || try_template_string(source, language, &mut cursor)
                || try_verbatim_string(source, ch, next, &mut cursor)
        };
        if matched {
            continue;
        }
        index += 1;
        col += 1;
    }
    out
}
