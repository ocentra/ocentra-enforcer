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
    if !source.is_char_boundary(*index)
        || !language.triple_double_strings
        || !source[*index..].starts_with("\"\"\"")
    {
        return false;
    }
    let Some(end) = source[*index + 3..].find("\"\"\"") else {
        return false;
    };
    let content = &source[*index + 3..*index + 3 + end];
    out.push(candidate(
        content,
        *line,
        *col,
        LiteralKind::Triple,
        line_at(source, *line),
    ));
    let consumed = 3 + end + 3;
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
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
    let ch = source.as_bytes()[*index] as char;
    if ch != '"' && !(language.single_quote_strings && ch == '\'') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(&source[*index..], ch) else {
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
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
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
    let ch = source.as_bytes()[*index] as char;
    if !(language.backtick_strings && ch == '`') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(&source[*index..], '`') else {
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
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
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
    let Some((content, consumed)) = read_quoted(&source[*index + 1..], '"') else {
        return false;
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Raw,
        line_at(source, *line),
    ));
    advance_position(&source[*index..*index + 1 + consumed], line, col);
    *index += 1 + consumed;
    true
}
