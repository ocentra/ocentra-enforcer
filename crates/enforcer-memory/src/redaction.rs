//! X06.8: community-export redaction.
//!
//! A [`enforcer_domain::memory_types::MemoryShareScope::Community`] bundle is the widest-audience
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

use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    MemoryRedactionIdentity, MemoryRedactionRepoRoot, MemoryRedactionSnippetLength,
    MemoryRedactionText, ParserSourceText,
};
use regex::Regex;

/// Default maximum length (in bytes) of any single raw-source/log
/// snippet surfaced in a community export before it is truncated with a
/// marker.
pub const DEFAULT_MAX_SNIPPET_LEN: usize = 400;

/// Redaction configuration. `max_snippet_len` is the only tunable knob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedactionConfig {
    pub max_snippet_len: MemoryRedactionSnippetLength,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            max_snippet_len: DEFAULT_MAX_SNIPPET_LEN.into(),
        }
    }
}

const PATH_MARKER: &str = "<repo-path>";
const IDENTITY_MARKER: &str = "<redacted-identity>";
const SECRET_MARKER: &str = "<redacted-secret>";
const TRUNCATION_MARKER: &str = "\n... [truncated for community export]";

/// Compile a `pattern` literal that is fixed and known-valid at review
/// time (every call site below passes a hand-written, syntax-checked
/// pattern). Invalid static syntax is a process-integrity failure, so
/// initialization fails closed instead of continuing without redaction.
fn static_regex(pattern: ParserSourceText<'_>) -> Regex {
    let Ok(regex) = Regex::new(pattern.as_str()) else {
        std::process::abort();
    };
    regex
}

// Absolute Windows path: drive letter + `:` + `\` or `/`.
static WINDOWS_ABS_PATH: LazyLock<Regex> = LazyLock::new(|| {
    static_regex(ParserSourceText::from(
        r#"[A-Za-z]:[\\/](?:[^\s\\/:*?"<>|]+[\\/])*[^\s\\/:*?"<>|]+"#,
    ))
});

// POSIX-style absolute path rooted at `/home/`, `/Users/`, or `/root/`.
static POSIX_HOME_PATH: LazyLock<Regex> =
    LazyLock::new(|| static_regex(ParserSourceText::from(r"/(?:home|Users|root)/[^\s]+")));

// `@handle`-shaped mention.
static AT_HANDLE: LazyLock<Regex> =
    LazyLock::new(|| static_regex(ParserSourceText::from(r"@[A-Za-z0-9_-]+")));

// Email address.
static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    static_regex(ParserSourceText::from(
        r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
    ))
});

/// Secret-shaped patterns: common API-key/token prefixes, generic
/// long-hex/encoded assignment patterns, and PEM private-key headers.
/// Deliberately basic/allowlist-style rather than a full entropy-based
/// secret scanner -- this is a redaction safety net for a community
/// export, not a standalone secret-scanning product.
static SECRET_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        static_regex(ParserSourceText::from(
            r"\b(?:sk-[A-Za-z0-9]{16,}|ghp_[A-Za-z0-9]{20,}|gho_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16})\b",
        )),
        static_regex(ParserSourceText::from(
            r#"(?i)\b(api[_-]?key|secret|token|password|passwd)\b\s*[:=]\s*['"][^'"\s]{8,}['"]"#,
        )),
        static_regex(ParserSourceText::from(
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
        )),
    ]
});

/// Rewrite absolute filesystem paths to a repo-relative, anonymized
/// form. A path under `repo_root` is rewritten relative to that root
/// (forward-slash normalized, root prefix + separator stripped); any
/// OTHER absolute path is replaced with [`PATH_MARKER`] rather than
/// guessed at.
pub fn redact_path(
    text: impl Into<MemoryRedactionText>,
    repo_root: Option<&MemoryRedactionRepoRoot>,
) -> MemoryRedactionText {
    let text = text.into();
    let mut out = text.as_str().retained();
    if let Some(root) = repo_root {
        out = strip_repo_root_prefix(
            ParserSourceText::from(out.as_str()),
            ParserSourceText::from(root.as_str()),
        )
        .as_str()
        // ALLOC-JUSTIFICATION: the path-redaction stage owns the rewritten
        // text before applying the remaining regex passes.
        .to_owned();
    }
    out = WINDOWS_ABS_PATH.replace_all(&out, PATH_MARKER).into_owned();
    out = POSIX_HOME_PATH.replace_all(&out, PATH_MARKER).into_owned();
    out.into()
}

/// Find every occurrence of `root` (in either `\`- or `/`-separated
/// form) followed by a path separator and a run of non-whitespace path
/// characters, and rewrite it to the forward-slash-normalized relative
/// remainder (root + separator stripped).
fn strip_repo_root_prefix(
    text: ParserSourceText<'_>,
    root: ParserSourceText<'_>,
) -> MemoryRedactionText {
    let root_escaped_fwd = regex::escape(&root.as_str().replace('\\', "/"));
    let root_escaped_back = regex::escape(&root.as_str().replace('/', "\\"));
    let pattern = format!(r#"(?:{root_escaped_fwd}|{root_escaped_back})[\\/]([^\s"'<>|,;)]*)"#);
    // `regex::escape` guarantees every metacharacter in `root` is
    // escaped, so this pattern -- built entirely from an escaped
    // caller-supplied string plus fixed, review-checked literal syntax
    // around it -- is always well-formed regardless of what `root`
    // contains. Failure must remain fail-closed because returning the
    // original text here could expose an absolute repository path.
    let Ok(re) = Regex::new(&pattern) else {
        std::process::abort();
    };
    re.replace_all(text.as_str(), |caps: &regex::Captures<'_>| {
        caps.get(1)
            .map_or("", |matched| matched.as_str())
            .replace('\\', "/")
    })
    .into_owned()
    .into()
}

/// Strip author-identity-shaped text: email addresses and `@handle`
/// mentions. `explicit_identities` is a caller-supplied list of exact
/// identity strings known from structured fields that must be redacted
/// even when they do not match the generic patterns.
pub fn redact_identity(
    text: impl Into<MemoryRedactionText>,
    explicit_identities: &[MemoryRedactionIdentity],
) -> MemoryRedactionText {
    let text = text.into();
    let mut out = EMAIL
        .replace_all(text.as_str(), IDENTITY_MARKER)
        .into_owned();
    out = AT_HANDLE.replace_all(&out, IDENTITY_MARKER).into_owned();
    for identity in explicit_identities {
        if identity.is_empty() {
            continue;
        }
        out = out.replace(identity.as_str(), IDENTITY_MARKER);
    }
    out.into()
}

/// Strip secret-shaped strings per [`SECRET_PATTERNS`].
pub fn redact_secrets(text: impl Into<MemoryRedactionText>) -> MemoryRedactionText {
    let text = text.into();
    let mut out = text.as_str().retained();
    for pattern in SECRET_PATTERNS.iter() {
        out = pattern.replace_all(&out, SECRET_MARKER).into_owned();
    }
    out.into()
}

/// Truncate `text` to at most `max_len` bytes at a UTF-8-safe boundary,
/// appending [`TRUNCATION_MARKER`] when truncation actually occurred.
pub fn truncate_snippet(
    text: impl Into<MemoryRedactionText>,
    max_len: MemoryRedactionSnippetLength,
) -> MemoryRedactionText {
    let text = text.into();
    let max_len = max_len.get();
    if text.len() <= max_len {
        return text;
    }
    let mut cut = max_len;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let Some(head) = text.get(..cut) else {
        return text;
    };
    let mut out = head.retained();
    out.push_str(TRUNCATION_MARKER);
    out.into()
}

/// Run the full community-export redaction pipeline over `text` in the
/// fixed order: secrets, then paths, then identities, then length
/// truncation.
pub fn redact_text(
    text: impl Into<MemoryRedactionText>,
    repo_root: Option<&MemoryRedactionRepoRoot>,
    explicit_identities: &[MemoryRedactionIdentity],
    config: RedactionConfig,
) -> MemoryRedactionText {
    let stage1 = redact_secrets(text);
    let stage2 = redact_path(stage1, repo_root);
    let stage3 = redact_identity(stage2, explicit_identities);
    truncate_snippet(stage3, config.max_snippet_len)
}

/// Redact one [`crate::record::MemoryRecord`] for community export:
/// `statement`/`why`/`how_to_apply` run through [`redact_text`];
/// `provenance.user`/`provenance.session_id`/`provenance.model` are
/// unconditionally cleared; `landed_at`/`evidence` path-shaped refs are
/// path-redacted too.
pub fn redact_record(
    record: &crate::record::MemoryRecord,
    repo_root: Option<&MemoryRedactionRepoRoot>,
    config: RedactionConfig,
) -> crate::record::MemoryRecord {
    let mut redacted = record.to_dto();
    let explicit_identities: Vec<MemoryRedactionIdentity> = [
        record.provenance().user.as_deref(),
        record.provenance().session_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(Into::into)
    .collect();

    redacted.statement =
        redact_text(record.statement(), repo_root, &explicit_identities, config).into();
    redacted.why = record
        .why()
        .map(|w| redact_text(w, repo_root, &explicit_identities, config).into());
    redacted.how_to_apply = record
        .how_to_apply()
        .map(|h| redact_text(h, repo_root, &explicit_identities, config).into());
    redacted.landed_at = record
        .landed_at()
        .iter()
        .map(|l| redact_path(l.as_str(), repo_root).into())
        .collect();
    if let Some(evidence) = redacted.evidence.as_mut() {
        evidence.r#ref = evidence
            .r#ref
            .as_ref()
            .map(|r| redact_path(r, repo_root).into());
    }

    // Author identity fields: unconditionally cleared, not
    // pattern-matched. `writer` (the lane/stream name) is kept: it
    // identifies a WORK STREAM, not a person.
    redacted.provenance.user = None;
    redacted.provenance.session_id = None;
    redacted.provenance.model = None;

    crate::record::MemoryRecord::from_dto(redacted)
}
