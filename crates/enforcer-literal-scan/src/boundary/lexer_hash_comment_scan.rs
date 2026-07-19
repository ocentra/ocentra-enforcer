use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted, LexerCursor};
use crate::{LanguageSpec, LiteralCandidate, LiteralKind};

pub(crate) fn lex_hash_comment(source: &str, language: LanguageSpec) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while let Some(byte) = bytes.get(index).copied() {
        let ch = char::from(byte);
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while let Some(comment_byte) = bytes.get(index).copied() {
                if comment_byte == b'\n' {
                    break;
                }
                index += 1;
                col += 1;
            }
            continue;
        }
        let matched = {
            let mut cursor = LexerCursor {
                out: &mut out,
                index: &mut index,
                line: &mut line,
                col: &mut col,
            };
            try_hash_triple_string(source, language, &mut cursor)
                || try_hash_string(source, language, &mut cursor)
                || try_hash_template(source, language, &mut cursor)
        };
        if matched {
            continue;
        }
        index += 1;
        col += 1;
    }
    out
}

fn try_hash_triple_string(
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
    let Some(after_opening) = rest.get(3..) else {
        return false;
    };
    let Some(end) = after_opening.find("\"\"\"") else {
        return false;
    };
    let Some(content) = after_opening.get(..end) else {
        return false;
    };
    cursor.out.push(candidate(
        content,
        *cursor.line,
        *cursor.col,
        LiteralKind::Triple,
        line_at(source, *cursor.line),
    ));
    let consumed = 3 + end + 3;
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, cursor.line, cursor.col);
    *cursor.index += consumed;
    true
}

fn try_hash_string(source: &str, language: LanguageSpec, cursor: &mut LexerCursor<'_>) -> bool {
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let Some(first_byte) = rest.as_bytes().first().copied() else {
        return false;
    };
    let ch = char::from(first_byte);
    if ch != '"' && !(language.syntax.supports_single_quote() && ch == '\'') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, ch) else {
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

fn try_hash_template(source: &str, language: LanguageSpec, cursor: &mut LexerCursor<'_>) -> bool {
    let Some(rest) = source.get(*cursor.index..) else {
        return false;
    };
    let Some(first_byte) = rest.as_bytes().first().copied() else {
        return false;
    };
    let ch = char::from(first_byte);
    if !(language.syntax.supports_backtick() && ch == '`') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, '`') else {
        return false;
    };
    cursor.out.push(candidate(
        &content,
        *cursor.line,
        *cursor.col,
        LiteralKind::Template,
        line_at(source, *cursor.line),
    ));
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, cursor.line, cursor.col);
    *cursor.index += consumed;
    true
}
