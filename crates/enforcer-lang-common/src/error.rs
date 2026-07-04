//! Typed parse errors for `enforcer-lang-common`. Currently just the
//! `DEFERRED(#<ref>)[revisit:<value>]` annotation grammar
//! (`crate::rules::deferred_work`): a malformed annotation is a distinct,
//! named failure mode rather than a generic string, mirroring
//! `enforcer-config`'s `ConfigLoadError` shape (see
//! `crates/enforcer-config/src/error.rs`).

/// Why a `DEFERRED(...)` annotation attached to a deferral marker did not
/// parse as a valid exemption. Carrying the exact annotation text lets a
/// caller show the offending token verbatim in a `Finding::detail`.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum DeferredAnnotationError {
    /// The annotation did not start with the required `DEFERRED(` token at
    /// all (this variant is mostly internal — callers should not construct
    /// a parse attempt unless the `DEFERRED` token was already found).
    #[error("annotation `{raw}` does not start with `DEFERRED(`")]
    NotDeferredForm {
        /// The raw annotation text that failed to match.
        raw: String,
    },

    /// The `#<ref>` component is missing its leading `#`, is empty, or the
    /// surrounding `(...)` is unterminated.
    #[error("annotation `{raw}` has a missing or empty `#<ref>` component (expected `DEFERRED(#<ref>)[revisit:<value>]`)")]
    MissingOrEmptyRef {
        /// The raw annotation text that failed to match.
        raw: String,
    },

    /// The `[revisit:<value>]` component is missing, malformed, or its
    /// `<value>` is empty.
    #[error("annotation `{raw}` has a missing or empty `[revisit:<value>]` component (expected `DEFERRED(#<ref>)[revisit:<value>]`)")]
    MissingOrEmptyRevisit {
        /// The raw annotation text that failed to match.
        raw: String,
    },
}

/// Result alias for the annotation parser.
pub type DeferredAnnotationResult<T> = std::result::Result<T, DeferredAnnotationError>;
