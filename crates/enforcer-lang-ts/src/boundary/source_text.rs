//! Raw line-oriented parsing primitives for the TS validator families
//! that detect a forbidden textual pattern rather than a structural/typed
//! condition (`source_scan`, `test_scan`, `tests_family`,
//! `generic_scanner`).
//!
//! BOUNDARY-INVARIANT: parse raw source text into branded source lines and
//! classified line roles before rule evaluation.
//! boundaryOwnerNote: enforcer-lang-ts owns line-oriented source decoding.
//! Negative malformed and truncated input coverage is exercised below.
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

use std::num::NonZeroU32;

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::telemetry_types::SourceLine;

#[derive(Debug, Clone, Copy)]
/// One line of source, 1-based, with context available to guards.
pub(crate) struct SourceTextLine<'a> {
    /// 1-based line number.
    pub(crate) number: SourceLine,
    /// The raw line text (no trailing newline).
    pub(crate) text: ValidationSource<'a>,
}

/// Iterate the 1-based lines of `source`.
pub(crate) fn lines(source: ValidationSource<'_>) -> impl Iterator<Item = SourceTextLine<'_>> {
    source
        .as_str()
        .lines()
        .scan(Some(NonZeroU32::MIN), |next_line, text| {
            let current = (*next_line)?;
            *next_line = current.checked_add(1);
            Some(SourceTextLine {
                number: SourceLine::try_new(current),
                text: ValidationSource::from_text(text),
            })
        })
}

/// True when `text` is (trimmed) a `//` line comment or inside a `/* */`
/// block — used to keep matchers from firing on comment-only mentions of a
/// forbidden pattern (e.g. this doc comment's own examples).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of source code versus comment-only text.
pub(crate) enum SourceLineRole {
    CommentOnly,
    Code,
}

/// Classify one source line for structural rule matching.
pub(crate) fn source_line_role(text: ValidationSource<'_>) -> SourceLineRole {
    let trimmed = text.as_str().trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
        SourceLineRole::CommentOnly
    } else {
        SourceLineRole::Code
    }
}

/// Byte index is a word boundary on the left: either at the very start of
/// `text`, or the preceding byte is not an identifier character.
fn left_word_boundary(text: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }
    match text
        .get(..index)
        .and_then(|prefix| prefix.chars().next_back())
    {
        Some(c) => !(c.is_alphanumeric() || c == '_' || c == '$'),
        None => true,
    }
}

/// Byte index (of the char AFTER the match) is a word boundary on the
/// right: either at the very end of `text`, or the following char is not an
/// identifier character.
fn right_word_boundary(text: &str, end: usize) -> bool {
    match text.get(end..).and_then(|suffix| suffix.chars().next()) {
        Some(c) => !(c.is_alphanumeric() || c == '_' || c == '$'),
        None => true,
    }
}

/// Find every occurrence of `needle` in `text` that sits on a word
/// boundary on BOTH sides (guards bare-identifier triggers like `any` from
/// matching inside `Company` or `anyOf`).
pub(crate) fn find_word(text: &str, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut start = 0usize;
    while let Some(rel) = text.get(start..).and_then(|suffix| suffix.find(needle)) {
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
pub(crate) fn find_literal(text: &str, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut start = 0usize;
    while let Some(rel) = text.get(start..).and_then(|suffix| suffix.find(needle)) {
        let idx = start + rel;
        hits.push(idx);
        start = idx + needle.len().max(1);
    }
    hits
}

/// Replace the contents of quoted TypeScript strings with spaces while
/// preserving the delimiters and code outside the strings. Lexical rules such
/// as TS-6.3 must not treat prose like `"as separate steps"` as a type cast.
pub(crate) fn mask_string_literals(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut delimiter = None;
    let mut escaped = false;
    for character in text.chars() {
        if let Some(active) = delimiter {
            if escaped {
                escaped = false;
                masked.push(' ');
            } else if character == '\\' {
                escaped = true;
                masked.push(' ');
            } else if character == active {
                delimiter = None;
                masked.push(character);
            } else {
                masked.push(' ');
            }
        } else if matches!(character, '\'' | '"' | '`') {
            delimiter = Some(character);
            masked.push(character);
        } else {
            masked.push(character);
        }
    }
    masked
}

/// Guard for the `!` non-null-assertion trigger: a bare `!` token is a
/// non-null assertion only when it follows an identifier/`)`/`]` character
/// directly (postfix position) and is NOT itself part of `!=`, `!==`, or a
/// prefix logical-not (`!foo`, `!(`, `! `, start-of-expression `!`).
pub(crate) fn find_non_null_assertions(text: &str) -> Vec<usize> {
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
        let prev = text
            .get(..idx)
            .and_then(|prefix| prefix.chars().next_back());
        let is_postfix = matches!(prev, Some(c) if c.is_alphanumeric() || c == '_' || c == '$' || c == ')' || c == ']');
        if is_postfix {
            hits.push(idx);
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::validation::ValidationSource;
    use enforcer_domain::telemetry_types::SourceLine;

    use super::{
        find_non_null_assertions, find_word, lines, mask_string_literals, source_line_role,
        SourceLineRole,
    };

    #[test]
    fn find_word_respects_boundaries() {
        assert_eq!(find_word("let x: any = 1;", "any"), vec![7]);
        assert_eq!(find_word("company anyOf", "any"), Vec::<usize>::new());
    }

    #[test]
    fn mask_string_literals_preserves_code_but_hides_prose() {
        let masked = mask_string_literals(
            r#"const detail = "as separate steps"; const cast = raw as Widget;"#,
        );
        assert!(!masked.contains("as separate steps"));
        assert!(masked.contains("raw as Widget"));
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
        assert_eq!(
            source_line_role(ValidationSource::from_text("  // any note")),
            SourceLineRole::CommentOnly
        );
        assert_eq!(
            source_line_role(ValidationSource::from_text("* any note")),
            SourceLineRole::CommentOnly
        );
        assert_eq!(
            source_line_role(ValidationSource::from_text("const x: any = 1;")),
            SourceLineRole::Code
        );
    }

    #[test]
    fn lines_are_one_based() -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let collected: Vec<_> = lines(ValidationSource::from_text("a\nb\nc")).collect();
        assert_eq!(
            collected[0].number,
            SourceLine::try_new(std::num::NonZeroU32::new(1).ok_or_else(|| {
                enforcer_domain::boundary::decode_error::DecodeError::new(
                    "sourceLine",
                    "line must be positive",
                )
            })?)
        );
        assert_eq!(
            collected[2].number,
            SourceLine::try_new(std::num::NonZeroU32::new(3).ok_or_else(|| {
                enforcer_domain::boundary::decode_error::DecodeError::new(
                    "sourceLine",
                    "line must be positive",
                )
            })?)
        );
        Ok(())
    }
}
