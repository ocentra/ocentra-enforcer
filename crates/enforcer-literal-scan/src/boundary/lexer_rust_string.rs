//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_rust_helpers::{is_rust_lifetime, rust_raw_prefix};
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted, LexerCursor};
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn update_block_comment_state(
    ch: char,
    next: Option<char>,
    block_depth: &mut usize,
    index: &mut usize,
    col: &mut usize,
) {
    if ch == '/' && next == Some('*') {
        *block_depth += 1;
        *index += 2;
        *col += 2;
    } else if ch == '*' && next == Some('/') {
        *block_depth -= 1;
        *index += 2;
        *col += 2;
    } else {
        *index += 1;
        *col += 1;
    }
}

pub(crate) fn try_rust_raw_string(
    source: &str,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    let Some((prefix_len, hash_count, kind)) = rust_raw_prefix(rest) else {
        return false;
    };
    let content_start = *index + prefix_len;
    let closing = format!("\"{}", "#".repeat(hash_count));
    let Some(content_source) = source.get(content_start..) else {
        return false;
    };
    let Some(end_rel) = content_source.find(&closing) else {
        return false;
    };
    let Some(content) = content_source.get(..end_rel) else {
        return false;
    };
    out.push(candidate(
        content,
        *line,
        *col,
        kind,
        line_at(source, *line),
    ));
    let consumed = prefix_len + end_rel + closing.len();
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, line, col);
    *index += consumed;
    true
}

pub(crate) fn try_rust_byte_string(
    source: &str,
    ch: char,
    next: Option<char>,
    cursor: &mut LexerCursor<'_>,
) -> bool {
    if !(ch == 'b' && next == Some('"')) {
        return false;
    }
    let Some(byte_string_source) = source.get(*cursor.index + 1..) else {
        return false;
    };
    let Some((content, consumed)) = read_quoted(byte_string_source, '"') else {
        return false;
    };
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        LiteralKind::Byte,
        line_at(source, *cursor.line),
    ));
    let Some(consumed_source) = source.get(*cursor.index..*cursor.index + 1 + consumed) else {
        return false;
    };
    advance_position(consumed_source, cursor.line, cursor.col);
    *cursor.index += 1 + consumed;
    true
}

pub(crate) fn try_rust_standard_string(
    source: &str,
    ch: char,
    cursor: &mut LexerCursor<'_>,
) -> bool {
    if ch != '"' {
        return false;
    }
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let Some((content, consumed)) = read_quoted(rest, '"') else {
        return false;
    };
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        LiteralKind::Normal,
        line_at(source, *cursor.line),
    ));
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, cursor.line, cursor.col);
    *cursor.index += consumed;
    true
}

pub(crate) fn try_rust_char_or_lifetime(
    source: &str,
    ch: char,
    index: &mut usize,
    col: &mut usize,
    line: &mut usize,
) -> bool {
    if ch != '\'' {
        return false;
    }
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    if is_rust_lifetime(rest) {
        *index += 1;
        *col += 1;
        return true;
    }
    if let Some((_content, consumed)) = read_quoted(rest, '\'') {
        let Some(consumed_source) = rest.get(..consumed) else {
            return false;
        };
        advance_position(consumed_source, line, col);
        *index += consumed;
        return true;
    }
    false
}
