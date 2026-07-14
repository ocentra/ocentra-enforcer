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
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    let prefix_len = python_string_prefix_len(rest);
    let quote_index = *index + prefix_len;
    let Some(quote_source) = rest.get(prefix_len..) else {
        return false;
    };
    let Some(quote) = quote_source.chars().next() else {
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
    let Some(quote_source) = source.get(quote_index..) else {
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
    let Some(prefix) = source.get(*index..*index + prefix_len) else {
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
    out.push(candidate(
        content,
        *line,
        *col,
        kind,
        line_at(source, *line),
    ));
    let consumed = prefix_len + 3 + end + 3;
    let Some(consumed_source) = source.get(*index..*index + consumed) else {
        return false;
    };
    advance_position(consumed_source, line, col);
    *index += consumed;
    *last_block_start = false;
    true
}

// Inherited from the standalone Tools/ocentra-literal-scan tool (arc-13
// fold-in preserves lexer behavior as-is; a param-struct refactor across
// the shared cursor-state signature is out of scope for this workpack).
#[allow(clippy::too_many_arguments)]
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
    let Some(quote_source) = source.get(quote_index..) else {
        return false;
    };
    let Some((content, consumed_quote)) = read_quoted(quote_source, quote) else {
        return false;
    };
    let Some(prefix) = source.get(*index..*index + prefix_len) else {
        return false;
    };
    let kind = if prefix_has_f(prefix) {
        LiteralKind::FString
    } else if prefix_len > 0 {
        LiteralKind::Raw
    } else {
        LiteralKind::Normal
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        kind,
        line_at(source, *line),
    ));
    let consumed = prefix_len + consumed_quote;
    let Some(consumed_source) = source.get(*index..*index + consumed) else {
        return false;
    };
    advance_position(consumed_source, line, col);
    *index += consumed;
    *last_block_start = false;
    true
}
