//! Python source-boundary line scanning used by every PY source/test-shape rule in
//! this crate builds on. Each rule is a small DATA record (markers +
//! position guard), not a bespoke struct — 61 near-identical hand-rolled
//! detectors would be the actual maintenance risk here, not this shared
//! engine.
//!
//! # The mem-arc-06-0002 gotcha this module exists to avoid
//! A naive "does this line contain substring X" scan double-fires for every
//! syntactic position a marker can appear in (a string literal mentioning
//! `"noqa"`, a docstring mentioning `except Exception`, a comment
//! mentioning `eval(`). [`Guard`] narrows a marker match to the syntactic
//! position the rule actually cares about (trailing comment, statement
//! keyword prefix, dict-literal assignment shape, ...) so the fail fixture
//! trips on the REAL pattern and the pass fixture -- which may legitimately
//! mention the marker in prose -- stays silent.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::PythonFindingMessage;

/// Narrows a raw marker match to the syntactic position a rule cares about.
/// Guards are intentionally line-local (this crate's validators inspect one
/// line at a time, never a parsed AST) but each still encodes a POSITION
/// check, not just "the marker is a substring somewhere" — that is what
/// keeps distinct rules from co-firing on the same incidental text.
#[derive(Debug, Clone, Copy)]
pub enum Guard {
    /// The marker may appear anywhere on the line except inside a `#`
    /// comment or a quoted string literal that starts before the marker.
    /// Used for statement-shaped markers (`except Exception`, `eval(`,
    /// `os.system(`) where a comment or string mentioning the same text is
    /// legitimately clean.
    NotInCommentOrString,
    /// The marker must appear as a trailing `#`-comment suffix on the line
    /// (e.g. `# noqa`, `# type: ignore`). Prose mentioning the same words
    /// mid-statement does not count.
    TrailingComment,
    /// The line, after trimming leading whitespace, must START with one of
    /// the markers (module-level statement shape: `import *`, `from x
    /// import *`, `CACHE = {}`).
    LineStartsWith,
    /// No positional narrowing beyond substring containment outside
    /// comments/strings — same as `NotInCommentOrString`, kept as a
    /// distinct name for call-site clarity where the rule is a plain
    /// "this API must never appear" ban.
    Anywhere,
    /// Like `NotInCommentOrString`, but additionally requires the marker
    /// not be preceded by an identifier character. Without this, a marker
    /// like `eval(` would also match inside `ast.literal_eval(` -- a
    /// DIFFERENT function that merely ends in the same letters. This is the
    /// mem-arc-06-0002 gotcha in its plainest form: matching "the marker is
    /// a substring" instead of "the marker is the call/keyword at this
    /// position".
    WordBoundary,
}

/// One rule's line-scan definition: the [`RuleId`] it proves, the guard
/// that scopes matches to the right syntactic position, and the marker
/// substrings that trip it.
#[derive(Debug)]
pub struct LineMarkerValidator {
    rule_id: RuleId,
    title: &'static str,
    guard: Guard,
    markers: &'static [&'static str],
}

impl LineMarkerValidator {
    /// Build a validator for one rule's marker set.
    pub(crate) fn new(
        rule_id: RuleId,
        title: &'static str,
        guard: Guard,
        markers: &'static [&'static str],
    ) -> Self {
        Self {
            rule_id,
            title,
            guard,
            markers,
        }
    }
}

impl Validator for LineMarkerValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (idx, line) in input.source.as_str().lines().enumerate() {
            if let Some(marker) = matches_line(line, self.guard, self.markers) {
                let Ok(line_number) = crate::boundary::source::one_based_line(idx) else {
                    continue;
                };
                if let Some(finding) = crate::boundary::finding::from_python_source(
                    &self.rule_id,
                    Severity::Error,
                    input.file,
                    line_number,
                    PythonFindingMessage::new(
                        self.title,
                        format!("matched marker `{marker}`"),
                        Some(line.trim()),
                    ),
                ) {
                    findings.push(finding);
                }
            }
        }
        findings
    }
}

/// A rule of the shape "this call MUST be paired with that companion
/// argument/keyword on the SAME line" (e.g. `requests.get(...)` must carry
/// `timeout=`). Fires when a `trigger` marker is present on a line but none
/// of `companions` are also present on that same line -- so a call that
/// legitimately includes the companion stays silent, while a bare call
/// trips it.
#[derive(Debug)]
pub struct MissingCompanionValidator {
    rule_id: RuleId,
    title: &'static str,
    triggers: &'static [&'static str],
    companions: &'static [&'static str],
}

impl MissingCompanionValidator {
    /// Build a validator for one rule's trigger/companion pair.
    pub(crate) fn new(
        rule_id: RuleId,
        title: &'static str,
        triggers: &'static [&'static str],
        companions: &'static [&'static str],
    ) -> Self {
        Self {
            rule_id,
            title,
            triggers,
            companions,
        }
    }
}

impl Validator for MissingCompanionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (idx, line) in input.source.as_str().lines().enumerate() {
            let code_part = code_before_comment(line);
            let has_trigger = self.triggers.iter().any(|t| code_part.contains(*t));
            if !has_trigger {
                continue;
            }
            let has_companion = self.companions.iter().any(|c| code_part.contains(*c));
            if has_companion {
                continue;
            }
            let Ok(line_number) = crate::boundary::source::one_based_line(idx) else {
                continue;
            };
            if let Some(finding) = crate::boundary::finding::from_python_source(
                &self.rule_id,
                Severity::Error,
                input.file,
                line_number,
                PythonFindingMessage::new(
                    self.title,
                    "required companion argument/keyword is missing on this line",
                    Some(line.trim()),
                ),
            ) {
                findings.push(finding);
            }
        }
        findings
    }
}

/// Fires on a BARE truthiness assertion (`assert user`, `assert result`)
/// -- a lone identifier with no comparison, attribute access, call, or
/// membership test -- while staying silent on `assert user.name == "x"` or
/// `assert result == 5`, which prove a concrete outcome. A naive
/// `LineStartsWith("assert user")` marker would also match the strong form
/// (it is a prefix of it), which is exactly the kind of position-blind
/// false positive this crate's fixtures are built to catch.
#[derive(Debug)]
pub struct WeakAssertionValidator {
    rule_id: RuleId,
    title: &'static str,
    identifiers: &'static [&'static str],
}

impl WeakAssertionValidator {
    /// Build a validator that flags bare `assert <identifier>` statements
    /// for the given candidate identifier names.
    pub(crate) fn new(
        rule_id: RuleId,
        title: &'static str,
        identifiers: &'static [&'static str],
    ) -> Self {
        Self {
            rule_id,
            title,
            identifiers,
        }
    }
}

impl Validator for WeakAssertionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (idx, line) in input.source.as_str().lines().enumerate() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("assert ") else {
                continue;
            };
            let is_bare_identifier = self.identifiers.contains(&rest);
            if !is_bare_identifier {
                continue;
            }
            let Ok(line_number) = crate::boundary::source::one_based_line(idx) else {
                continue;
            };
            if let Some(finding) = crate::boundary::finding::from_python_source(
                &self.rule_id,
                Severity::Error,
                input.file,
                line_number,
                PythonFindingMessage::new(
                    self.title,
                    format!("bare truthiness assertion `assert {rest}`"),
                    Some(trimmed),
                ),
            ) {
                findings.push(finding);
            }
        }
        findings
    }
}

/// Find the first marker (if any) that matches `line` under `guard`.
fn matches_line<'a>(line: &str, guard: Guard, markers: &'a [&'a str]) -> Option<&'a str> {
    let trimmed = line.trim_start();
    match guard {
        Guard::LineStartsWith => markers
            .iter()
            .find(|marker| trimmed.starts_with(**marker))
            .copied(),
        Guard::TrailingComment => {
            let comment = trailing_comment(line)?;
            markers
                .iter()
                .find(|marker| comment.contains(**marker))
                .copied()
        }
        Guard::NotInCommentOrString | Guard::Anywhere => {
            let code_part = code_before_comment(line);
            markers
                .iter()
                .find(|marker| code_part.contains(**marker))
                .copied()
        }
        Guard::WordBoundary => {
            let code_part = code_before_comment(line);
            markers
                .iter()
                .find(|marker| contains_at_word_boundary(code_part, marker))
                .copied()
        }
    }
}

/// Like `str::contains`, but the match must not be immediately preceded by
/// an identifier character (`[A-Za-z0-9_]`) -- so `eval(` matches
/// `x = eval(expr)` but not `ast.literal_eval(expr)`.
fn contains_at_word_boundary(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(rel_idx) = haystack.get(start..).and_then(|tail| tail.find(needle)) {
        let idx = start + rel_idx;
        let boundary_ok = idx == 0
            || bytes
                .get(idx.saturating_sub(1))
                .is_none_or(|byte| !is_identifier_byte(*byte));
        if boundary_ok {
            return true;
        }
        start = idx + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Return the `#`-comment suffix of a line, if it has one, excluding the
/// leading `#`. A `#` inside a quoted string is not treated as starting a
/// comment (best-effort: scans for the first unquoted `#`).
fn trailing_comment(line: &str) -> Option<&str> {
    let hash_idx = first_unquoted_char(line, '#')?;
    line.get(hash_idx.saturating_add(1)..)
}

/// Return the portion of `line` before any `#` comment starts (best-effort
/// quote tracking so `"a # b"` is not mistaken for a comment start).
fn code_before_comment(line: &str) -> &str {
    match first_unquoted_char(line, '#') {
        Some(idx) => line.get(..idx).unwrap_or(line),
        None => line,
    }
}

/// Byte index of the first occurrence of `needle` that is not inside a
/// single- or double-quoted string literal on this line. Line-local
/// best-effort tracking (no multi-line string/triple-quote awareness) —
/// sufficient for the fixture-proven marker shapes this crate scans for.
fn first_unquoted_char(line: &str, needle: char) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let bytes = line.as_bytes();
    for (idx, ch) in line.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c == needle && !in_single && !in_double => return Some(idx),
            _ => {}
        }
        let _ = bytes;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{matches_line, Guard};

    #[test]
    fn trailing_comment_guard_ignores_prose_mid_statement() {
        let markers: &[&str] = &["noqa"];
        assert_eq!(
            matches_line("x = 1  # noqa", Guard::TrailingComment, markers),
            Some("noqa")
        );
        assert!(matches_line(
            "msg = \"do not use noqa in prod\"",
            Guard::TrailingComment,
            markers
        )
        .is_none());
    }

    #[test]
    fn not_in_comment_or_string_guard_ignores_comment_mentions() {
        let markers: &[&str] = &["except Exception"];
        assert_eq!(
            matches_line("except Exception:", Guard::NotInCommentOrString, markers),
            Some("except Exception")
        );
        assert!(matches_line(
            "# do not write except Exception here",
            Guard::NotInCommentOrString,
            markers
        )
        .is_none());
    }

    #[test]
    fn line_starts_with_guard_requires_leading_position() {
        let markers: &[&str] = &["import *"];
        assert_eq!(
            matches_line("import *", Guard::LineStartsWith, markers),
            Some("import *")
        );
        assert!(matches_line("x = \"import *\"", Guard::LineStartsWith, markers).is_none());
    }
}
