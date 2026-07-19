//! Typed parse-at-boundary error for [`super::parse`] (h08,
//! POLICY-SPEC-INGESTION).
//!
//! Every malformed-input path returns a variant here — never a silent
//! default, never a panic/unwrap. `thiserror` derives `Display`/`Error` so
//! callers can surface the reason without reaching into private fields.

/// Everything that can go wrong turning a project's `.mdc` spec text into a
/// typed [`super::PolicySpec`](super::PolicySpec).
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum PolicyIngestError {
    /// The spec text has no recognizable section headers at all (e.g.
    /// empty input, binary garbage, or a document that is not shaped like
    /// the ingestable `.mdc` convention).
    #[error("policy spec `{spec_source}` has no recognizable `## <Section>` headers: {reason}")]
    NoSections {
        /// Which input this error is about (fixture path or logical name).
        spec_source: String,
        /// Human-readable reason.
        reason: String,
    },
    /// A required section is present but its list-item lines could not be
    /// parsed into entries (e.g. a rule-assertion line missing its rule id
    /// token).
    #[error("policy spec `{spec_source}` section `{section}` has a malformed entry: {reason}")]
    MalformedEntry {
        /// Which input this error is about.
        spec_source: String,
        /// The section header under which the malformed entry was found.
        section: String,
        /// Human-readable reason.
        reason: String,
    },
    /// The spec asserts the same rule id twice with conflicting severities
    /// — ambiguous input, not silently resolved by "last wins".
    #[error(
        "policy spec `{spec_source}` asserts rule `{rule_id}` twice with conflicting severities \
         (`{first}` vs `{second}`)"
    )]
    ConflictingSeverity {
        /// Which input this error is about.
        spec_source: String,
        /// The rule id asserted twice.
        rule_id: String,
        /// First severity token seen.
        first: String,
        /// Second, conflicting severity token seen.
        second: String,
    },
}
