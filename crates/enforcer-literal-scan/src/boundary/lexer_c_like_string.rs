use crate::lexer_import_context::is_import_specifier_context;
use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted, LexerCursor};
use crate::{LanguageSpec, LiteralKind};

pub(crate) fn try_triple_string(
    source: &str,
    language: LanguageSpec,
    cursor: &mut LexerCursor<'_>,
) -> bool {
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    if !language.syntax.supports_triple_double() || !rest.starts_with("\"\"\"") {
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
    cursor.out.push(candidate(
        content,
        *cursor.line,
        *cursor.col,
        LiteralKind::Triple,
        line_at(source, *cursor.line),
    ));
    let Some(consumed) = end.checked_add(6) else {
        return false;
    };
    let Some(consumed_text) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_text, cursor.line, cursor.col);
    let Some(next_index) = (*cursor.index).checked_add(consumed) else {
        return false;
    };
    *cursor.index = next_index;
    true
}

pub(crate) fn try_standard_string(
    source: &str,
    language: LanguageSpec,
    ts_mode: bool,
    cursor: &mut LexerCursor<'_>,
) -> bool {
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let Some(ch) = rest.chars().next() else {
        return false;
    };
    if ch != '"' && !(language.syntax.supports_single_quote() && ch == '\'') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, ch) else {
        return false;
    };
    let mut kind = LiteralKind::Normal;
    if ts_mode
        && is_import_specifier_context(
            line_at(source, *cursor.line).as_deref().unwrap_or(""),
            &content,
        )
    {
        kind = LiteralKind::ImportSpecifier;
    }
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        kind,
        line_at(source, *cursor.line),
    ));
    let Some(consumed_text) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_text, cursor.line, cursor.col);
    let Some(next_index) = (*cursor.index).checked_add(consumed) else {
        return false;
    };
    *cursor.index = next_index;
    true
}

pub(crate) fn try_template_string(
    source: &str,
    language: LanguageSpec,
    cursor: &mut LexerCursor<'_>,
) -> bool {
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let Some(ch) = rest.chars().next() else {
        return false;
    };
    if !(language.syntax.supports_backtick() && ch == '`') {
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
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        kind,
        line_at(source, *cursor.line),
    ));
    let Some(consumed_text) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_text, cursor.line, cursor.col);
    let Some(next_index) = (*cursor.index).checked_add(consumed) else {
        return false;
    };
    *cursor.index = next_index;
    true
}

pub(crate) fn try_verbatim_string(
    source: &str,
    ch: char,
    next: Option<char>,
    cursor: &mut LexerCursor<'_>,
) -> bool {
    if !(source.is_char_boundary(*cursor.index) && ch == '@' && next == Some('"')) {
        return false;
    }
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let Some(after_at) = rest.strip_prefix('@') else {
        return false;
    };
    let Some((content, consumed)) = read_quoted(after_at, '"') else {
        return false;
    };
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        LiteralKind::Raw,
        line_at(source, *cursor.line),
    ));
    let Some(total) = consumed.checked_add(1) else {
        return false;
    };
    let Some(consumed_text) = rest.get(..total) else {
        return false;
    };
    advance_position(consumed_text, cursor.line, cursor.col);
    let Some(next_index) = (*cursor.index).checked_add(total) else {
        return false;
    };
    *cursor.index = next_index;
    true
}
