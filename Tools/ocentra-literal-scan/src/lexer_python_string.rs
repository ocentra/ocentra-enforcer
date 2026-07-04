use crate::lexer_python_prefix::{prefix_has_f, python_string_prefix_len};
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LiteralCandidate, LiteralKind};

pub(crate) fn try_python_string(
    source: &str,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
    last_block_start: &mut bool,
) -> bool {
    let prefix_len = if source.is_char_boundary(*index) {
        python_string_prefix_len(&source[*index..])
    } else {
        0
    };
    let quote_index = *index + prefix_len;
    if !source.is_char_boundary(quote_index) {
        return false;
    }
    let Some(quote) = source[quote_index..].chars().next() else {
        return false;
    };
    if quote != '"' && quote != '\'' {
        return false;
    }
    if try_python_triple_string(
        source,
        prefix_len,
        quote_index,
        quote,
        out,
        index,
        line,
        col,
        last_block_start,
    ) {
        return true;
    }
    try_python_quoted_string(
        source,
        prefix_len,
        quote_index,
        quote,
        out,
        index,
        line,
        col,
        last_block_start,
    )
}

pub(crate) fn current_line_ends_block(source: &str, line: usize) -> bool {
    line_at(source, line)
        .unwrap_or_default()
        .trim_end()
        .ends_with(':')
}

fn try_python_triple_string(
    source: &str,
    prefix_len: usize,
    quote_index: usize,
    quote: char,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
    last_block_start: &mut bool,
) -> bool {
    let delimiter = format!("{quote}{quote}{quote}");
    if !source[quote_index..].starts_with(&delimiter) {
        return false;
    }
    let Some(end) = source[quote_index + 3..].find(&delimiter) else {
        return false;
    };
    let content = &source[quote_index + 3..quote_index + 3 + end];
    let mut kind = if prefix_has_f(&source[*index..*index + prefix_len]) {
        LiteralKind::FString
    } else {
        LiteralKind::Triple
    };
    if *last_block_start {
        kind = LiteralKind::DocString;
    }
    out.push(candidate(content, *line, *col, kind, line_at(source, *line)));
    let consumed = prefix_len + 3 + end + 3;
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
    *last_block_start = false;
    true
}

fn try_python_quoted_string(
    source: &str,
    prefix_len: usize,
    quote_index: usize,
    quote: char,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
    last_block_start: &mut bool,
) -> bool {
    let Some((content, consumed_quote)) = read_quoted(&source[quote_index..], quote) else {
        return false;
    };
    let kind = if prefix_has_f(&source[*index..*index + prefix_len]) {
        LiteralKind::FString
    } else if prefix_len > 0 {
        LiteralKind::Raw
    } else {
        LiteralKind::Normal
    };
    out.push(candidate(&content, *line, *col, kind, line_at(source, *line)));
    let consumed = prefix_len + consumed_quote;
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
    *last_block_start = false;
    true
}
