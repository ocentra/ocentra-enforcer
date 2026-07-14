use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
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
        if try_hash_triple_string(source, language, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        if try_hash_string(source, language, &mut out, &mut index, &mut line, &mut col) {
            continue;
        }
        if try_hash_template(source, language, &mut out, &mut index, &mut line, &mut col) {
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
    let Some(after_opening) = rest.get(3..) else {
        return false;
    };
    let Some(end) = after_opening.find("\"\"\"") else {
        return false;
    };
    let Some(content) = after_opening.get(..end) else {
        return false;
    };
    out.push(candidate(
        content,
        *line,
        *col,
        LiteralKind::Triple,
        line_at(source, *line),
    ));
    let consumed = 3 + end + 3;
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, line, col);
    *index += consumed;
    true
}

fn try_hash_string(
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
    let Some(first_byte) = rest.as_bytes().first().copied() else {
        return false;
    };
    let ch = char::from(first_byte);
    if ch != '"' && !(language.single_quote_strings && ch == '\'') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, ch) else {
        return false;
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Normal,
        line_at(source, *line),
    ));
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, line, col);
    *index += consumed;
    true
}

fn try_hash_template(
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
    let Some(first_byte) = rest.as_bytes().first().copied() else {
        return false;
    };
    let ch = char::from(first_byte);
    if !(language.backtick_strings && ch == '`') {
        return false;
    }
    let Some((content, consumed)) = read_quoted(rest, '`') else {
        return false;
    };
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Template,
        line_at(source, *line),
    ));
    let Some(consumed_source) = rest.get(..consumed) else {
        return false;
    };
    advance_position(consumed_source, line, col);
    *index += consumed;
    true
}
