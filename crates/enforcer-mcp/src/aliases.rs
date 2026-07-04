//! Legacy `rust_rules_*` alias surface + defined deprecation window.
//!
//! Ported from the legacy `.mjs` registry/dispatch pair
//! (`mcp/rust-rules-mcp-tool-registry.mjs`'s `LEGACY_ALIAS_TOOLS`,
//! `mcp/rust-rules-mcp-fingerprint.mjs`'s `normalizeToolName`): every
//! canonical `ocentra_enforcer_*` tool is doubled under a `rust_rules_*`
//! name, and dispatch folds any `rust_rules_*` call back to its canonical
//! handler before lookup. This is a DEFINED, one-Rust-pack-release
//! deprecation window (workpack row), not a permanent surface — see
//! [`deprecation_window_open`].

use crate::name::{CANONICAL_TOOL_PREFIX, LEGACY_ALIAS_PREFIX};

/// Whether the legacy alias surface is currently ADVERTISED and DISPATCHED.
///
/// TRANSITIONAL: `true` for the "one Rust-pack compatibility release" the
/// workpack names. Flip to `false` (or remove this seam entirely,
/// coordinating with x03) once that release closes — [`crate::registry`]
/// and [`crate::router`] both consult this single flag rather than each
/// hardcoding the window state.
pub fn deprecation_window_open() -> bool {
    true
}

/// Derive a tool's `rust_rules_*` alias name from its canonical
/// `ocentra_enforcer_*` name.
///
/// # Panics
/// Never — a name lacking the canonical prefix is returned unchanged
/// (defensive; callers only ever pass [`crate::registry::CANONICAL_TOOLS`]
/// entries, which always carry the prefix).
pub fn alias_name(canonical: &str) -> String {
    match canonical.strip_prefix(CANONICAL_TOOL_PREFIX) {
        Some(rest) => format!("{LEGACY_ALIAS_PREFIX}{rest}"),
        None => canonical.to_owned(),
    }
}

/// Fold any `rust_rules_*` name back to its canonical `ocentra_enforcer_*`
/// form. Names already canonical (or unrelated) pass through unchanged.
/// Mirrors `normalizeToolName` verbatim.
pub fn normalize_tool_name(name: &str) -> String {
    match name.strip_prefix(LEGACY_ALIAS_PREFIX) {
        Some(rest) => format!("{CANONICAL_TOOL_PREFIX}{rest}"),
        None => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{alias_name, deprecation_window_open, normalize_tool_name};
    use crate::registry::CANONICAL_TOOLS;

    #[test]
    fn alias_name_derives_rust_rules_prefix() {
        assert_eq!(alias_name("ocentra_enforcer_check"), "rust_rules_check");
        assert_eq!(
            alias_name("ocentra_enforcer_coordination_claim"),
            "rust_rules_coordination_claim"
        );
    }

    #[test]
    fn normalize_tool_name_folds_alias_back_to_canonical() {
        assert_eq!(
            normalize_tool_name("rust_rules_check"),
            "ocentra_enforcer_check"
        );
        assert_eq!(
            normalize_tool_name("ocentra_enforcer_check"),
            "ocentra_enforcer_check"
        );
        assert_eq!(normalize_tool_name("some_other_tool"), "some_other_tool");
    }

    #[test]
    fn every_canonical_tool_round_trips_through_alias_and_back() {
        for &canonical in CANONICAL_TOOLS {
            let alias = alias_name(canonical);
            assert!(alias.starts_with("rust_rules_"));
            assert_eq!(normalize_tool_name(&alias), canonical);
        }
    }

    #[test]
    fn deprecation_window_is_currently_open() {
        // Pass fixture for "alias resolves + appears in tools/list" — see
        // crate::registry::build_tool_descriptors and crate::router tests
        // for the full end-to-end fixtures this flag gates.
        assert!(deprecation_window_open());
    }
}
