use crate::lexer_shared::{advance_position, candidate, line_at, read_quoted};
use crate::{LanguageSpec, LiteralCandidate, LiteralKind};

pub(crate) fn lex_hash_comment(source: &str, language: LanguageSpec) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while index < bytes.len() && bytes[index] as char != '\n' {
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

fn try_hash_string(
    source: &str,
    language: LanguageSpec,
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

fn try_hash_template(
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
    out.push(candidate(
        &content,
        *line,
        *col,
        LiteralKind::Template,
        line_at(source, *line),
    ));
    advance_position(&source[*index..*index + consumed], line, col);
    *index += consumed;
    true
}
