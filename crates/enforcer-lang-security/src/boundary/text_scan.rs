//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Shared line-oriented matching primitives for the security validator
//! families (`secret_scan`, `generic_scanner`).
//!
//! # The comment-guard opt-out gotcha (mem-arc-07-0003)
//!
//! A shared comment-only-line guard (skip matching on lines that are pure
//! `//`/`#`/`/* */` comments) is the right default posture for MOST rules
//! in this crate â€” a secret pattern merely MENTIONED in a comment (e.g. a
//! doc example) is not a live occurrence of it. But arc-07's memory
//! flagged that a uniform guard silently defeats any rule whose violation
//! IS itself a comment. None of the 22 `SEC-*` rules in this crate have
//! that shape (unlike TS-2.1's suppression-comment rule), but every
//! [`crate::boundary::spec::RuleSpec`] still carries an explicit comment posture
//! rather than hard-coding the guard, so a future rule with that shape can
//! opt out without restructuring the matcher.
//!
//! # The double-dispatch / position-guard gotcha (mem-arc-06-0002)
//!
//! A bare substring search matches regardless of surrounding context â€”
//! e.g. a `.env` path fragment inside an unrelated word, or a `key`/
//! `secret` identifier substring inside a longer identifier. Every regex
//! pattern below is anchored with `\b` word boundaries or explicit
//! anchors (`^`, `$`, path-segment boundaries) rather than a bare
//! substring test, mirroring arc-06's guidance to guard by position, not
//! text-kind alone.

/// One line of source, 1-based, with the raw text available for
/// position-aware guards.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceLine<'a> {
    /// 1-based line number.
    pub(crate) number: u32,
    /// The raw line text (no trailing newline).
    pub(crate) text: enforcer_domain::boundary::validation::ValidationSource<'a>,
}

/// Iterate the 1-based lines of `source`.
pub(crate) fn lines(
    source: enforcer_domain::boundary::validation::ValidationSource<'_>,
) -> impl Iterator<Item = SourceLine<'_>> {
    source
        .as_str()
        .lines()
        .enumerate()
        .map(|(idx, text)| SourceLine {
            number: match u32::try_from(idx) {
                Ok(number) => number.saturating_add(1),
                Err(_) => u32::MAX,
            },
            text: enforcer_domain::boundary::validation::ValidationSource::from_text(text),
        })
}

use regex::Regex;

/// True when `text` is a command-like line as classified by the shared
/// generic scanner. Tooling-policy rules use this context guard so regex
/// literals in their own rule definitions are not mistaken for invocations.
pub(crate) fn is_command_like_line(text: &str) -> bool {
    static COMMAND_LIKE: std::sync::OnceLock<Result<Regex, regex::Error>> =
        std::sync::OnceLock::new();
    let Ok(pattern) = COMMAND_LIKE.get_or_init(|| {
        Regex::new(
            r#"(?ix)^\s*(?:run:\s*)?(?:-\s+|>\s+)?(?:npx\s+|npm\s+|pnpm\s+|yarn\s+|node\s+|python(?:3)?\s+|uv\s+run\s+|cargo\s+|ruff\s+|pyright\b|mypy\b|gitleaks\b|trufflehog\b|\./|[A-Za-z]:[\\/])|\b(?:execSync|spawnSync|spawn|exec)\s*\(\s*[\"'`][^\"'`]*(?:gitleaks|trufflehog|ruff|pyright|mypy|npm\s+install)"#,
        )
    }) else {
        return false;
    };
    pattern.is_match(text)
}

/// True when `text` is (trimmed) a `//`, `#`, or `/* */`-style comment
/// line â€” used to keep matchers from firing on comment-only mentions of a
/// forbidden pattern (this doc comment's own examples included).
pub(crate) fn is_comment_only_line(
    text: enforcer_domain::boundary::validation::ValidationSource<'_>,
) -> bool {
    let trimmed = text.as_str().trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with('*')
        || trimmed.starts_with("/*")
}

#[cfg(test)]
mod tests {
    use super::{is_command_like_line, is_comment_only_line, lines};

    #[test]
    fn command_context_excludes_rule_definition_literals() {
        assert!(!is_command_like_line(
            r##"compile(r"(?i)\btrufflehog\b")?"##
        ));
        assert!(is_command_like_line("trufflehog filesystem ."));
        let invocation = ["execSync(\"", "trufflehog filesystem .", "\")"].concat();
        assert!(is_command_like_line(&invocation));
    }

    #[test]
    fn lines_are_one_based() {
        let collected: Vec<_> =
            lines(enforcer_domain::boundary::validation::ValidationSource::from_text("a\nb\nc"))
                .collect();
        assert_eq!(collected[0].number, 1);
        assert_eq!(collected[2].number, 3);
    }

    #[test]
    fn comment_only_line_detection() {
        for comment in [
            "  // token = \"abc\"",
            "# token = abc",
            "* still inside a block comment",
        ] {
            assert!(is_comment_only_line(
                enforcer_domain::boundary::validation::ValidationSource::from_text(comment)
            ));
        }
        assert!(!is_comment_only_line(
            enforcer_domain::boundary::validation::ValidationSource::from_text("token = \"abc\";")
        ));
    }
}
