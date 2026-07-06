//! X06.8: community-export redaction.
//!
//! A `community`-[`crate::share::Scope`] bundle is the widest-audience
//! export tier this crate produces, so it is the one that MUST NOT leak:
//!
//! - absolute repo paths (rewritten to a repo-root-relative, then
//!   anonymized form -- see [`redact_path`]);
//! - author identities (`provenance.user`, `provenance.session_id`,
//!   free-text `@handle`-shaped mentions -- see [`redact_identity`]);
//! - secret-shaped strings (API keys, tokens, connection strings, private
//!   key headers -- see [`redact_secrets`]);
//! - raw source text beyond a configured snippet length (long code/log
//!   blocks are truncated, never silently included in full -- see
//!   [`truncate_snippet`]).
//!
//! [`redact_text`] composes all four passes in a fixed order (secrets
//! first -- a secret embedded in what looks like a path or identity must
//! still be caught -- then paths, then identities, then length
//! truncation last, since truncation must see the final redacted text's
//! length, not the pre-redaction length) so the same input always
//! produces the same output: this is deliberately a pure function with
//! no randomness and no locale/timezone dependence, because the golden
//! test in this module's test suite asserts byte-exact output.

use std::sync::LazyLock;

use regex::Regex;

/// Default maximum length (in bytes) of any single raw-source/log
/// snippet surfaced in a community export before it is truncated with a
/// marker. Kept generous enough to preserve useful context but short
/// enough that a community bundle can never smuggle out a whole file.
pub const DEFAULT_MAX_SNIPPET_LEN: usize = 400;

/// Redaction configuration. `max_snippet_len` is the only tunable knob
/// (the path/identity/secret passes are not configurable -- they are
/// safety invariants, not preferences).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedactionConfig {
    pub max_snippet_len: usize,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            max_snippet_len: DEFAULT_MAX_SNIPPET_LEN,
        }
    }
}

/// Marker text substituted for anything this module strips, so a
/// redacted document still shows readers THAT something was removed
/// (never silently blank, which would look like the field was simply
/// empty to begin with).
const PATH_MARKER: &str = "<repo-path>";
const IDENTITY_MARKER: &str = "<redacted-identity>";
const SECRET_MARKER: &str = "<redacted-secret>";
const TRUNCATION_MARKER: &str = "\n... [truncated for community export]";

/// Compile a `pattern` literal that is fixed and known-valid at review
/// time (every call site below passes a hand-written, syntax-checked
/// pattern). Matches the same "defensive fallback keeps the constructor
/// infallible without `.unwrap()`/`.expect()`" idiom
/// [`crate::ids::ArtifactId::from_content`] already uses in this crate:
/// the error path is unreachable in practice (these patterns are fixed
/// string literals, not user input), so it is spelled as
/// `unreachable!()` rather than `.expect(...)` -- clippy's
/// `unwrap_used`/`expect_used` lints (workspace-denied) target exactly
/// those two calls, not the `unreachable!()` macro.
fn static_regex(pattern: &str) -> Regex {
    Regex::new(pattern)
        .unwrap_or_else(|_| unreachable!("static regex pattern {pattern:?} is always valid"))
}

// Absolute Windows path: drive letter + `:` + `\` or `/`, e.g.
// `C:\Projects\enforcer\src\lib.rs` or `C:/Projects/enforcer/src/lib.rs`.
static WINDOWS_ABS_PATH: LazyLock<Regex> = LazyLock::new(|| {
    static_regex(r#"[A-Za-z]:[\\/](?:[^\s\\/:*?"<>|]+[\\/])*[^\s\\/:*?"<>|]+"#)
});

// POSIX-style absolute path rooted at `/home/`, `/Users/`, or `/root/`
// (paths most likely to embed a real username), kept deliberately
// narrower than "any string starting with /" so ordinary repo-relative
// or URL-path-shaped text is not falsely flagged.
static POSIX_HOME_PATH: LazyLock<Regex> =
    LazyLock::new(|| static_regex(r"/(?:home|Users|root)/[^\s]+"));

// `@handle`-shaped mention (chat/VCS-style author mention).
static AT_HANDLE: LazyLock<Regex> = LazyLock::new(|| static_regex(r"@[A-Za-z0-9_-]+"));

// Email address.
static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| static_regex(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"));

/// Secret-shaped patterns: common API-key/token prefixes, generic
/// long-hex/base64-ish assignment patterns, and PEM private-key headers.
/// Deliberately basic/allowlist-style (per the mission's "basic
/// patterns" scope) rather than a full entropy-based secret scanner --
/// this is a redaction safety net for a community export, not a
/// standalone secret-scanning product.
static SECRET_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Common vendor-prefixed API keys/tokens (GitHub, Slack, AWS,
        // Anthropic/OpenAI-style, generic `sk-`/`ghp_`/`xox` families).
        static_regex(
            r"\b(?:sk-[A-Za-z0-9]{16,}|ghp_[A-Za-z0-9]{20,}|gho_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16})\b",
        ),
        // `key = "..."` / `token: '...'` / `password="..."` style
        // assignments with a long opaque value.
        static_regex(
            r#"(?i)\b(api[_-]?key|secret|token|password|passwd)\b\s*[:=]\s*['"][^'"\s]{8,}['"]"#,
        ),
        // PEM private-key block headers (the header line alone is
        // enough to redact -- catching the whole block is the caller's
        // responsibility if it appears across multiple lines).
        static_regex(r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    ]
});

/// Rewrite absolute filesystem paths to a repo-relative, anonymized
/// form. A path under `repo_root` is rewritten relative to that root
/// (forward-slash normalized, root prefix + separator stripped); any
/// OTHER absolute path (outside the known repo root, or matched by the
/// generic absolute-path patterns when no repo root is known) is
/// replaced with [`PATH_MARKER`] rather than guessed at.
pub fn redact_path(text: &str, repo_root: Option<&str>) -> String {
    let mut out = text.to_owned();
    if let Some(root) = repo_root {
        out = strip_repo_root_prefix(&out, root);
    }
    out = WINDOWS_ABS_PATH.replace_all(&out, PATH_MARKER).into_owned();
    out = POSIX_HOME_PATH
        .replace_all(&out, PATH_MARKER)
        .into_owned();
    out
}

/// Find every occurrence of `root` (in either `\`- or `/`-separated
/// form) followed by a path separator and a run of non-whitespace path
/// characters, and rewrite it to the forward-slash-normalized relative
/// remainder (root + separator stripped). Matching is done on the raw
/// bytes of `text` rather than via a single blind string-replace of the
/// root prefix so that ONLY the remainder after the root is
/// separator-normalized -- text before/after the matched span is left
/// exactly as written.
fn strip_repo_root_prefix(text: &str, root: &str) -> String {
    let root_escaped_fwd = regex::escape(&root.replace('\\', "/"));
    let root_escaped_back = regex::escape(&root.replace('/', "\\"));
    let pattern =
        format!(r#"(?:{root_escaped_fwd}|{root_escaped_back})[\\/]([^\s"'<>|,;)]*)"#);
    // `regex::escape` guarantees every metacharacter in `root` is
    // escaped, so this pattern -- built entirely from an escaped
    // caller-supplied string plus the fixed, review-checked literal
    // syntax around it -- is always well-formed regardless of what
    // `root` contains; same infallible-by-construction idiom as
    // `static_regex` above.
    let re = Regex::new(&pattern)
        .unwrap_or_else(|_| unreachable!("escaped repo-root pattern is always valid"));
    re.replace_all(text, |caps: &regex::Captures<'_>| {
        caps[1].replace('\\', "/")
    })
    .into_owned()
}

/// Strip author-identity-shaped text: email addresses and `@handle`
/// mentions. `explicit_identities` is a caller-supplied list of exact
/// identity strings known from structured fields (e.g.
/// `provenance.user`, `provenance.session_id`) that must be redacted
/// even when they do not match the generic patterns (a bare username
/// with no `@`/domain, for example).
pub fn redact_identity(text: &str, explicit_identities: &[&str]) -> String {
    let mut out = EMAIL.replace_all(text, IDENTITY_MARKER).into_owned();
    out = AT_HANDLE.replace_all(&out, IDENTITY_MARKER).into_owned();
    for identity in explicit_identities {
        if identity.is_empty() {
            continue;
        }
        out = out.replace(identity, IDENTITY_MARKER);
    }
    out
}

/// Strip secret-shaped strings per [`SECRET_PATTERNS`].
pub fn redact_secrets(text: &str) -> String {
    let mut out = text.to_owned();
    for pattern in SECRET_PATTERNS.iter() {
        out = pattern.replace_all(&out, SECRET_MARKER).into_owned();
    }
    out
}

/// Truncate `text` to at most `max_len` bytes at a UTF-8-safe boundary,
/// appending [`TRUNCATION_MARKER`] when truncation actually occurred.
/// Text at or under the limit is returned unchanged (no marker appended
/// -- the marker's presence itself signals "this was cut").
pub fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_owned();
    }
    let mut cut = max_len;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = text[..cut].to_owned();
    out.push_str(TRUNCATION_MARKER);
    out
}

/// Run the full community-export redaction pipeline over `text` in the
/// fixed order the module docs specify: secrets, then paths, then
/// identities, then length truncation.
pub fn redact_text(
    text: &str,
    repo_root: Option<&str>,
    explicit_identities: &[&str],
    config: RedactionConfig,
) -> String {
    let stage1 = redact_secrets(text);
    let stage2 = redact_path(&stage1, repo_root);
    let stage3 = redact_identity(&stage2, explicit_identities);
    truncate_snippet(&stage3, config.max_snippet_len)
}

/// Redact one [`crate::record::MemoryRecord`] for community export:
/// `statement`/`why`/`how_to_apply` run through [`redact_text`];
/// `provenance.user`/`provenance.session_id`/`provenance.model` are
/// unconditionally cleared (author identity has no legitimate reason to
/// survive into a community-tier bundle, so this is not merely pattern
/// redaction -- the fields are dropped outright); `landed_at`/`evidence`
/// path-shaped refs are path-redacted too.
pub fn redact_record(
    record: &crate::record::MemoryRecord,
    repo_root: Option<&str>,
    config: RedactionConfig,
) -> crate::record::MemoryRecord {
    let mut redacted = record.clone();
    let explicit_identities: Vec<&str> = [
        record.provenance.user.as_deref(),
        record.provenance.session_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();

    redacted.statement = redact_text(&record.statement, repo_root, &explicit_identities, config);
    redacted.why = record
        .why
        .as_ref()
        .map(|w| redact_text(w, repo_root, &explicit_identities, config));
    redacted.how_to_apply = record
        .how_to_apply
        .as_ref()
        .map(|h| redact_text(h, repo_root, &explicit_identities, config));
    redacted.landed_at = record
        .landed_at
        .iter()
        .map(|l| redact_path(l, repo_root))
        .collect();
    if let Some(evidence) = redacted.evidence.as_mut() {
        evidence.r#ref = evidence
            .r#ref
            .as_ref()
            .map(|r| redact_path(r, repo_root));
    }

    // Author identity fields: unconditionally cleared, not
    // pattern-matched -- a community export carries no writer identity
    // at all. `writer` (the lane/stream name, e.g. "arc-05") is kept: it
    // identifies a WORK STREAM, not a person, and the workpack's own
    // schema treats it as non-identity metadata.
    redacted.provenance.user = None;
    redacted.provenance.session_id = None;
    redacted.provenance.model = None;

    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Evidence, MemoryRecord, Provenance, RecordDomain, RecordKind};

    #[test]
    fn redacts_windows_and_posix_absolute_paths() {
        let text = r"see C:\Projects\enforcer\src\lib.rs and /home/alice/notes.txt";
        let out = redact_path(text, None);
        assert!(!out.contains("Projects"));
        assert!(!out.contains("alice"));
        assert!(out.contains(PATH_MARKER));
    }

    #[test]
    fn redacts_repo_root_relative_paths_by_stripping_the_root() {
        let text = r"crash in C:\Projects\enforcer\src\lib.rs line 4";
        let out = redact_path(text, Some(r"C:\Projects\enforcer"));
        assert_eq!(out, "crash in src/lib.rs line 4");
    }

    #[test]
    fn redacts_emails_and_handles() {
        let text = "reported by alice@example.com, cc @bob-dev";
        let out = redact_identity(text, &[]);
        assert!(!out.contains("alice@example.com"));
        assert!(!out.contains("@bob-dev"));
        assert_eq!(out.matches(IDENTITY_MARKER).count(), 2);
    }

    #[test]
    fn redacts_explicit_identity_strings_even_without_pattern_match() {
        let text = "session owned by sujan.mishra on this box";
        let out = redact_identity(text, &["sujan.mishra"]);
        assert!(!out.contains("sujan.mishra"));
    }

    #[test]
    fn redacts_secret_shaped_strings() {
        let cases = [
            "token: sk-abcdefghijklmnopqrstuvwx",
            "ghp_abcdefghijklmnopqrstuvwxyz012345",
            r#"api_key = "abcdef1234567890""#,
            "-----BEGIN RSA PRIVATE KEY-----",
        ];
        for case in cases {
            let out = redact_secrets(case);
            assert!(
                out.contains(SECRET_MARKER),
                "expected secret redaction for {case:?}, got {out:?}"
            );
        }
    }

    #[test]
    fn leaves_ordinary_text_untouched() {
        let text = "this rule fires on missing error handling in async functions";
        assert_eq!(redact_secrets(text), text);
        assert_eq!(redact_path(text, None), text);
        assert_eq!(redact_identity(text, &[]), text);
    }

    #[test]
    fn truncates_beyond_configured_length_with_marker() {
        let long = "x".repeat(1000);
        let out = truncate_snippet(&long, 100);
        assert!(out.len() < long.len());
        assert!(out.ends_with(TRUNCATION_MARKER));
        assert_eq!(&out[..100], &long[..100]);
    }

    #[test]
    fn short_text_is_unchanged_by_truncation() {
        let short = "short snippet";
        assert_eq!(truncate_snippet(short, 400), short);
    }

    #[test]
    fn redact_record_clears_identity_fields_and_redacts_paths() {
        let record = MemoryRecord {
            schema_version: 1,
            id: "mem-primary-0001".to_string(),
            ts: "2026-07-05T00:00:00Z".to_string(),
            kind: RecordKind::Lesson,
            domain: RecordDomain::Harness,
            statement: r"fix landed in C:\Projects\enforcer\src\lib.rs, reported by alice@example.com"
                .to_string(),
            why: None,
            how_to_apply: None,
            applies_to: vec![],
            evidence: Some(Evidence {
                source: Some("gitHistory".to_string()),
                r#ref: Some(r"C:\Projects\enforcer\src\lib.rs".to_string()),
            }),
            routes: vec![],
            landed_at: vec![r"C:\Projects\enforcer\src\lib.rs".to_string()],
            supersedes: None,
            provenance: Provenance {
                writer: "arc-05".to_string(),
                session_id: Some("agent-abc123".to_string()),
                model: Some("claude-sonnet-5".to_string()),
                user: Some("sujan.mishra".to_string()),
            },
        };

        let redacted = redact_record(&record, Some(r"C:\Projects\enforcer"), RedactionConfig::default());
        assert!(redacted.provenance.user.is_none());
        assert!(redacted.provenance.session_id.is_none());
        assert!(redacted.provenance.model.is_none());
        assert_eq!(redacted.provenance.writer, "arc-05");
        assert!(!redacted.statement.contains("Projects"));
        assert!(!redacted.statement.contains("alice@example.com"));
        assert_eq!(redacted.landed_at[0], "src/lib.rs");
        assert_eq!(
            redacted.evidence.as_ref().and_then(|e| e.r#ref.as_deref()),
            Some("src/lib.rs")
        );
    }

    /// GOLDEN: fixture bundle in -> byte-exact expected redacted output.
    /// The input and expected-output fixtures are committed under
    /// `tests/fixtures/memory/redaction/` so a future change to the
    /// redaction pipeline that alters output shape is caught as a diff
    /// against a committed, reviewable fixture rather than an inline
    /// string only visible in this test file.
    #[test]
    fn golden_community_export_redaction_is_byte_exact() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/memory/redaction");
        let input = std::fs::read_to_string(fixture_dir.join("community-input.ndjson"))
            .expect("read golden input fixture");
        let expected = std::fs::read_to_string(fixture_dir.join("community-expected.ndjson"))
            .expect("read golden expected fixture");

        let record: MemoryRecord =
            serde_json::from_str(input.trim_end()).expect("parse golden input record");
        let redacted = redact_record(
            &record,
            Some(r"C:\Projects\enforcer"),
            RedactionConfig::default(),
        );
        let actual =
            serde_json::to_string(&redacted).expect("serialize redacted record") + "\n";
        assert_eq!(
            actual, expected,
            "community redaction output must be byte-exact against the committed golden fixture"
        );
    }
}
