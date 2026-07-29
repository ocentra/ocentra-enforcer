//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Raw syntax-token observations at the Rust parser boundary.

/// Check a parsed identifier suffix without leaking an allocated raw string
/// into rule-domain code.
pub(crate) fn ident_ends_with(ident: &syn::Ident, suffix: &str) -> bool {
    ident.to_string().ends_with(suffix)
}
