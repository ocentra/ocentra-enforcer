use crate::lexer_import_context::is_import_specifier_context;
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LanguageSpec, LiteralCandidate, LiteralKind};

pub(crate) fn try_triple_string(
    source: &str,
    language: LanguageSpec,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    if !language.triple_double_strings || !rest.starts_with("\"\"\"") {
        return false;
    }
    let Some(after_open) = rest.strip_prefix("\"\"\"") else {
        return false;
    };
    let Some(end) = after_open.find("\"\"\"") else {
        return false;
    };
    let Some(content) = after_open.get(..end) else {
        return false;
    };
    out.push(candidate(
        content,
        *line,
        *col,
        LiteralKind::Triple,
        line_at(source, *line),
    ));
    let Some(consumed) = end.checked_add(6) else {
        return false;
    };
    let Some(consumed_text) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_text, line, col);
    let Some(next_index) = (*index).checked_add(consumed) else {
        return false;
    };
    *index = next_index;
    true
}

pub(crate) fn try_standard_string(
    source: &str,
    language: LanguageSpec,
    ts_mode: bool,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    let Some(ch) = rest.chars().next() else {
        return false;
    };
    if ch != '"' && !(language.single_quote_strings && ch == '\'') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, ch) else {
        return false;
    };
    let mut kind = LiteralKind::Normal;
    if ts_mode
        && is_import_specifier_context(line_at(source, *line).as_deref().unwrap_or(""), &content)
    {
        kind = LiteralKind::ImportSpecifier;
    }
    out.push(candidate(
        &content,
        *line,
        *col,
        kind,
        line_at(source, *line),
    ));
    let Some(consumed_text) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_text, line, col);
    let Some(next_index) = (*index).checked_add(consumed) else {
        return false;
    };
    *index = next_index;
    true
}

pub(crate) fn try_template_string(
    source: &str,
    language: LanguageSpec,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    let Some(ch) = rest.chars().next() else {
        return false;
    };
    if !(language.backtick_strings && ch == '`') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, '`') else {
        return false;
    };
    let kind = if content.contains("${") {
        LiteralKind::InterpolatedTemplate
    } else {
        LiteralKind::Template
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        kind,
        line_at(source, *line),
    ));
    let Some(consumed_text) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_text, line, col);
    let Some(next_index) = (*index).checked_add(consumed) else {
        return false;
    };
    *index = next_index;
    true
}

pub(crate) fn try_verbatim_string(
    source: &str,
    ch: char,
    next: Option<char>,
    out: &mut Vec<LiteralCandidate>,
    index: &mut usize,
    line: &mut usize,
    col: &mut usize,
) -> bool {
    if !(source.is_char_boundary(*index) && ch == '@' && next == Some('"')) {
        return false;
    }
    let Some(rest) = source.get(*index..) else {
        return false;
    };
    let Some(after_at) = rest.strip_prefix('@') else {
        return false;
    };
    let Some((content, consumed)) = read_quoted(after_at, '"') else {
        return false;
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Raw,
        line_at(source, *line),
    ));
    let Some(total) = consumed.checked_add(1) else {
        return false;
    };
    let Some(consumed_text) = rest.get(..total) else {
        return false;
    };
    advance_position(consumed_text, line, col);
    let Some(next_index) = (*index).checked_add(total) else {
        return false;
    };
    *index = next_index;
    true
}
