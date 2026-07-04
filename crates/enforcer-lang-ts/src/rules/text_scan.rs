//! Shared line-oriented matching primitives for the TS validator families
//! that detect a forbidden textual pattern rather than a structural/typed
//! condition (`source_scan`, `test_scan`, `tests_family`,
//! `generic_scanner`).
//!
//! # The double-dispatch gotcha (mem-arc-06-0002)
//!
//! arc-06's memory flagged that AST-visitor-style matching fires for every
//! node of a given syntactic kind regardless of WHERE that node sits — a
//! bare "does this line contain the substring" check has the same failure
//! mode: `!` (non-null assertion) also matches inside `!=`, `!==`, or a
//! logical-not `if (!ok)`; `let` (single-assignment binding) matches every
//! `let` including ones that ARE reassigned later. Every matcher in this
//! module is therefore guarded by POSITION (token boundaries, surrounding
//! character class, line role) and not just "does the byte sequence
//! appear", mirroring arc-06's guidance to guard by position, not node/text
//! kind alone.

/// One line of source, 1-based, with leading/trailing context available for
/// position-aware guards.
#[derive(Debug, Clone, Copy)]
pub struct SourceLine<'a> {
    /// 1-based line number.
    pub number: u32,
    /// The raw line text (no trailing newline).
    pub text: &'a str,
}

/// Iterate the 1-based lines of `source`.
pub fn lines(source: &str) -> impl Iterator<Item = SourceLine<'_>> {
    source.lines().enumerate().map(|(idx, text)| SourceLine {
        number: (idx as u32).saturating_add(1),
        text,
    })
}

/// True when `text` is (trimmed) a `//` line comment or inside a `/* */`
/// block — used to keep matchers from firing on comment-only mentions of a
/// forbidden pattern (e.g. this doc comment's own examples).
pub fn is_comment_only_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*")
}

/// Byte index is a word boundary on the left: either at the very start of
/// `text`, or the preceding byte is not an identifier character.
fn left_word_boundary(text: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }
    match text[..index].chars().next_back() {
        Some(c) => !(c.is_alphanumeric() || c == '_' || c == '$'),
        None => true,
    }
}

/// Byte index (of the char AFTER the match) is a word boundary on the
/// right: either at the very end of `text`, or the following char is not an
/// identifier character.
fn right_word_boundary(text: &str, end: usize) -> bool {
    match text[end..].chars().next() {
        Some(c) => !(c.is_alphanumeric() || c == '_' || c == '$'),
        None => true,
    }
}

/// Find every occurrence of `needle` in `text` that sits on a word
/// boundary on BOTH sides (guards bare-identifier triggers like `any` from
/// matching inside `Company` or `anyOf`).
pub fn find_word(text: &str, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut start = 0usize;
    while let Some(rel) = text[start..].find(needle) {
        let idx = start + rel;
        let end = idx + needle.len();
        if left_word_boundary(text, idx) && right_word_boundary(text, end) {
            hits.push(idx);
        }
        start = idx + needle.len().max(1);
    }
    hits
}

/// Find every occurrence of `needle` as a bare substring (no word-boundary
/// guard) — used for multi-character operators/punctuation sequences where
/// "word boundary" is meaningless (e.g. `as unknown as`, `export *`).
pub fn find_literal(text: &str, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut start = 0usize;
    while let Some(rel) = text[start..].find(needle) {
        let idx = start + rel;
        hits.push(idx);
        start = idx + needle.len().max(1);
    }
    hits
}

/// Guard for the `!` non-null-assertion trigger: a bare `!` token is a
/// non-null assertion only when it follows an identifier/`)`/`]` character
/// directly (postfix position) and is NOT itself part of `!=`, `!==`, or a
/// prefix logical-not (`!foo`, `!(`, `! `, start-of-expression `!`).
pub fn find_non_null_assertions(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut hits = Vec::new();
    for (idx, ch) in text.char_indices() {
        if ch != '!' {
            continue;
        }
        // Not `!=`/`!==`: next char must not be `=`.
        if bytes.get(idx + 1) == Some(&b'=') {
            continue;
        }
        // Postfix guard: previous non-space char must be an
        // identifier/`)`/`]` character — i.e. this `!` follows an
        // expression, not introduces one.
        let prev = text[..idx].chars().next_back();
        let is_postfix = matches!(prev, Some(c) if c.is_alphanumeric() || c == '_' || c == '$' || c == ')' || c == ']');
        if is_postfix {
            hits.push(idx);
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::{find_non_null_assertions, find_word, is_comment_only_line, lines};

    #[test]
    fn find_word_respects_boundaries() {
        assert_eq!(find_word("let x: any = 1;", "any"), vec![7]);
        assert_eq!(find_word("company anyOf", "any"), Vec::<usize>::new());
    }

    #[test]
    fn non_null_assertion_guards_against_not_equal_and_logical_not() {
        assert_eq!(
            find_non_null_assertions("if (!ok) return;"),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_non_null_assertions("if (a !== b) {}"),
            Vec::<usize>::new()
        );
        assert_eq!(
            find_non_null_assertions("if (a != b) {}"),
            Vec::<usize>::new()
        );
        let hits = find_non_null_assertions("const x = maybe!.value;");
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn comment_only_line_detection() {
        assert!(is_comment_only_line("  // any note"));
        assert!(is_comment_only_line("* any note"));
        assert!(!is_comment_only_line("const x: any = 1;"));
    }

    #[test]
    fn lines_are_one_based() {
        let collected: Vec<_> = lines("a\nb\nc").collect();
        assert_eq!(collected[0].number, 1);
        assert_eq!(collected[2].number, 3);
    }
}
