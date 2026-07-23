//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::lexer_python_prefix::{prefix_has_f, python_string_prefix_len};
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted, LexerCursor};
use crate::LiteralKind;

#[derive(Clone, Copy)]
struct PythonStringStart {
    prefix_len: usize,
    quote_index: usize,
    quote: char,
}

pub(crate) fn try_python_string(
    source: &str,
    cursor: &mut LexerCursor<'_>,
    last_block_start: &mut bool,
) -> bool {
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let prefix_len = python_string_prefix_len(rest);
    let quote_index = *cursor.index + prefix_len;
    let Some(quote_source) = rest.get(prefix_len..) else {
        return false;
    };
    let Some(quote) = quote_source.chars().next() else {
        return false;
    };
    if quote != '"' && quote != '\'' {
        return false;
    }
    let start = PythonStringStart {
        prefix_len,
        quote_index,
        quote,
    };
    if try_python_triple_string(source, start, cursor, last_block_start) {
        return true;
    }
    try_python_quoted_string(source, start, cursor, last_block_start)
}

pub(crate) fn current_line_ends_block(source: &str, line: usize) -> bool {
    line_at(source, line).is_some_and(|text| text.trim_end().ends_with(':'))
}

fn try_python_triple_string(
    source: &str,
    start: PythonStringStart,
    cursor: &mut LexerCursor<'_>,
    last_block_start: &mut bool,
) -> bool {
    let delimiter = format!("{0}{0}{0}", start.quote);
    let Some(quote_source) = source.get(start.quote_index..) else {
        return false;
    };
    if !quote_source.starts_with(&delimiter) {
        return false;
    }
    let Some(after_delimiter) = quote_source.get(3..) else {
        return false;
    };
    let Some(end) = after_delimiter.find(&delimiter) else {
        return false;
    };
    let Some(content) = after_delimiter.get(..end) else {
        return false;
    };
    let Some(prefix) = source.get(*cursor.index..*cursor.index + start.prefix_len) else {
        return false;
    };
    let mut kind = if prefix_has_f(prefix) {
        LiteralKind::FString
    } else {
        LiteralKind::Triple
    };
    if *last_block_start {
        kind = LiteralKind::DocString;
    }
    cursor.out.push(candidate(
        content,
        *cursor.line,
        *cursor.col,
        kind,
        line_at(source, *cursor.line),
    ));
    let consumed = start.prefix_len + 3 + end + 3;
    let Some(consumed_source) = source.get(*cursor.index..*cursor.index + consumed) else {
        return false;
    };
    advance_position(consumed_source, cursor.line, cursor.col);
    *cursor.index += consumed;
    *last_block_start = false;
    true
}

fn try_python_quoted_string(
    source: &str,
    start: PythonStringStart,
    cursor: &mut LexerCursor<'_>,
    last_block_start: &mut bool,
) -> bool {
    let Some(quote_source) = source.get(start.quote_index..) else {
        return false;
    };
    let Some((content, consumed_quote)) = read_quoted(quote_source, start.quote) else {
        return false;
    };
    let Some(prefix) = source.get(*cursor.index..*cursor.index + start.prefix_len) else {
        return false;
    };
    let kind = if prefix_has_f(prefix) {
        LiteralKind::FString
    } else if start.prefix_len > 0 {
        LiteralKind::Raw
    } else {
        LiteralKind::Normal
    };
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        kind,
        line_at(source, *cursor.line),
    ));
    let consumed = start.prefix_len + consumed_quote;
    let Some(consumed_source) = source.get(*cursor.index..*cursor.index + consumed) else {
        return false;
    };
    advance_position(consumed_source, cursor.line, cursor.col);
    *cursor.index += consumed;
    *last_block_start = false;
    true
}
