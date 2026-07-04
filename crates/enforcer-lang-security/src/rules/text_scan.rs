//! Shared line-oriented matching primitives for the security validator
//! families (`secret_scan`, `generic_scanner`).
//!
//! # The comment-guard opt-out gotcha (mem-arc-07-0003)
//!
//! A shared comment-only-line guard (skip matching on lines that are pure
//! `//`/`#`/`/* */` comments) is the right default posture for MOST rules
//! in this crate — a secret pattern merely MENTIONED in a comment (e.g. a
//! doc example) is not a live occurrence of it. But arc-07's memory
//! flagged that a uniform guard silently defeats any rule whose violation
//! IS itself a comment. None of the 22 `SEC-*` rules in this crate have
//! that shape (unlike TS-2.1's suppression-comment rule), but every
//! [`super::spec::RuleSpec`] still carries an explicit `comment_guard: bool`
//! rather than hard-coding the guard, so a future rule with that shape can
//! opt out without restructuring the matcher.
//!
//! # The double-dispatch / position-guard gotcha (mem-arc-06-0002)
//!
//! A bare substring search matches regardless of surrounding context —
//! e.g. a `.env` path fragment inside an unrelated word, or a `key`/
//! `secret` identifier substring inside a longer identifier. Every regex
//! pattern below is anchored with `\b` word boundaries or explicit
//! anchors (`^`, `$`, path-segment boundaries) rather than a bare
//! substring test, mirroring arc-06's guidance to guard by position, not
//! text-kind alone.

/// One line of source, 1-based, with the raw text available for
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

/// True when `text` is (trimmed) a `//`, `#`, or `/* */`-style comment
/// line — used to keep matchers from firing on comment-only mentions of a
/// forbidden pattern (this doc comment's own examples included).
pub fn is_comment_only_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with('*')
        || trimmed.starts_with("/*")
}

#[cfg(test)]
mod tests {
    use super::{is_comment_only_line, lines};

    #[test]
    fn lines_are_one_based() {
        let collected: Vec<_> = lines("a\nb\nc").collect();
        assert_eq!(collected[0].number, 1);
        assert_eq!(collected[2].number, 3);
    }

    #[test]
    fn comment_only_line_detection() {
        assert!(is_comment_only_line("  // token = \"abc\""));
        assert!(is_comment_only_line("# token = abc"));
        assert!(is_comment_only_line("* still inside a block comment"));
        assert!(!is_comment_only_line("token = \"abc\";"));
    }
}
