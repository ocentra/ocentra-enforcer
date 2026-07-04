use crate::lexer_rust_helpers::{is_rust_lifetime, rust_raw_prefix};
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
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
    if !source.is_char_boundary(*index) {
        return false;
    }
    let Some((prefix_len, hash_count, kind)) = rust_raw_prefix(&source[*index..]) else {
        return false;
    };
    let content_start = *index + prefix_len;
    let closing = format!("\"{}", "#".repeat(hash_count));
    let Some(end_rel) = source[content_start..].find(&closing) else {
        return false;
    };
    let content = &source[content_start..content_start + end_rel];
    out.push(candidate(
        content,
        *line,
        *col,
        kind,
        line_at(source, *line),
    ));
    let consumed = prefix_len + end_rel + closing.len();
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
    true
}

// Inherited from the standalone Tools/ocentra-literal-scan tool (arc-13
// fold-in preserves lexer behavior as-is; a param-struct refactor across
// the shared cursor-state signature is out of scope for this workpack).
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_rust_byte_string(
    source: &str,
    ch: char,
    next: Option<char>,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    if !(ch == 'b' && next == Some('"')) {
        return false;
    }
    let Some((content, consumed)) = read_quoted(&source[*index + 1..], '"') else {
        return false;
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Byte,
        line_at(source, *line),
    ));
    advance_position(&source[*index..*index + 1 + consumed], line, col);
    *index += 1 + consumed;
    true
}

// Inherited from the standalone Tools/ocentra-literal-scan tool (arc-13
// fold-in preserves lexer behavior as-is; a param-struct refactor across
// the shared cursor-state signature is out of scope for this workpack).
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_rust_standard_string(
    source: &str,
    ch: char,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    if ch != '"' {
        return false;
    }
    let Some((content, consumed)) = read_quoted(&source[*index..], '"') else {
        return false;
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Normal,
        line_at(source, *line),
    ));
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
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
    let rest = &source[*index..];
    if is_rust_lifetime(rest) {
        *index += 1;
        *col += 1;
        return true;
    }
    if let Some((_content, consumed)) = read_quoted(rest, '\'') {
        advance_position(&source[*index..*index + consumed], line, col);
        *index += consumed;
        return true;
    }
    false
}
