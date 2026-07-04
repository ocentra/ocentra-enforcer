//! Shared line-oriented matching primitives for the IaC validator families.
//! Mirrors `enforcer-lang-ts`'s `text_scan` module (same double-dispatch
//! position guard discipline — mem-arc-06-0002 — applies here too: matching
//! "does this line contain the literal substring" without regard to WHICH
//! block it sits in would fire outside the resource/block it is meant to
//! guard).
//!
//! # Comment guards (workpack SIBLING GOTCHAS note)
//! Every IaC validator here goes through [`RuleSpec::comment_guard`] (see
//! `super::spec`) so a `#`/`//` comment mentioning a forbidden token does
//! not itself trip the rule — matching `enforcer-lang-ts`'s posture.

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

/// True when `text` is (trimmed) a `#` (Terraform/YAML) or `//` (JSON has
/// no comments, but templates sometimes carry `//` in embedded policy
/// strings) line comment — used to keep matchers from firing on
/// comment-only mentions of a forbidden pattern.
pub fn is_comment_only_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('#') || trimmed.starts_with("//")
}

/// Find every occurrence of `needle` as a bare substring — used for the
/// HCL/JSON/YAML block and key tokens this crate matches (no identifier
/// word-boundary concept applies to `"Action": "*"`-shaped needles the way
/// it does to a bare keyword).
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

#[cfg(test)]
mod tests {
    use super::{find_literal, is_comment_only_line, lines};

    #[test]
    fn find_literal_finds_bare_substrings() {
        assert_eq!(
            find_literal("cidr_blocks = [\"0.0.0.0/0\"]", "0.0.0.0/0"),
            vec![16]
        );
        assert_eq!(
            find_literal("cidr_blocks = [\"10.0.0.0/16\"]", "0.0.0.0/0"),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn comment_only_line_detection() {
        assert!(is_comment_only_line("  # encrypt = true"));
        assert!(is_comment_only_line("// aws_secret_access_key = \"x\""));
        assert!(!is_comment_only_line("encrypt = true"));
    }

    #[test]
    fn lines_are_one_based() {
        let collected: Vec<_> = lines("a\nb\nc").collect();
        assert_eq!(collected[0].number, 1);
        assert_eq!(collected[2].number, 3);
    }
}
