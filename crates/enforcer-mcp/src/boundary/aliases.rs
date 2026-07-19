//! MCP boundary legacy `rust_rules_*` alias surface + defined deprecation window.
//!
//! Ported from the legacy `.mjs` registry/dispatch pair
//! (`mcp/rust-rules-mcp-tool-registry.mjs`'s `LEGACY_ALIAS_TOOLS`,
//! `mcp/rust-rules-mcp-fingerprint.mjs`'s `normalizeToolName`): every
//! canonical `ocentra_enforcer_*` tool is doubled under a `rust_rules_*`
//! name, and dispatch folds any `rust_rules_*` call back to its canonical
//! handler before lookup. This is a DEFINED, one-Rust-pack-release
//! deprecation window (workpack row), not a permanent surface — see
//! [`deprecation_window_open`].

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::mcp_types::McpToolName;

/// The canonical tool-name-family prefix this alias mechanism folds
/// against. Lives here (not in `crate::name`, x01's owned name-surface
/// file) because it is tied 1:1 to `crate::registry::CANONICAL_TOOLS`'s
/// own `ocentra_enforcer_*` literal family, which x01 does not own or
/// rewrite (see the workpack's `owns:` line) — x01's grep gate scans
/// `crate::name` (plus `Cargo.toml` name fields) and must find zero
/// matches there; this internal tool-family literal is unaffected by the
/// workpack's product-identity rename and keeps its historical value
/// here.
const CANONICAL_TOOL_PREFIX: &str = "ocentra_enforcer";

/// The legacy compatibility alias prefix. Lives here for the same reason
/// as [`CANONICAL_TOOL_PREFIX`] above. `pub(crate)` so [`crate::router`]
/// can recognize a closed-window alias call without this module needing
/// to re-export it as a public product-identity const.
pub(crate) const LEGACY_ALIAS_PREFIX: &str = "rust_rules";

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
pub fn alias_name(canonical: &McpToolName) -> Result<McpToolName, DecodeError> {
    match canonical.as_str().strip_prefix(CANONICAL_TOOL_PREFIX) {
        Some(rest) => McpToolName::try_new(&format!("{LEGACY_ALIAS_PREFIX}{rest}")),
        None => McpToolName::try_new(canonical.as_str()),
    }
}

/// Fold any `rust_rules_*` name back to its canonical `ocentra_enforcer_*`
/// form. Names already canonical (or unrelated) pass through unchanged.
/// Mirrors `normalizeToolName` verbatim.
/// PROPERTY-TEST: `tests/alias_properties.rs` generates canonical, alias,
/// and unrelated tool-name families and asserts normalization behavior.
pub fn normalize_tool_name(name: &McpToolName) -> Result<McpToolName, DecodeError> {
    match name.as_str().strip_prefix(LEGACY_ALIAS_PREFIX) {
        Some(rest) => McpToolName::try_new(&format!("{CANONICAL_TOOL_PREFIX}{rest}")),
        None => McpToolName::try_new(name.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::{alias_name, deprecation_window_open, normalize_tool_name};
    use crate::registry::CANONICAL_TOOLS;
    use enforcer_domain::mcp_types::McpToolName;

    fn tool(
        value: &str,
    ) -> Result<McpToolName, enforcer_domain::boundary::decode_error::DecodeError> {
        McpToolName::try_new(value)
    }

    #[test]
    fn alias_name_derives_rust_rules_prefix() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            alias_name(&tool("ocentra_enforcer_check")?)?.as_str(),
            "rust_rules_check"
        );
        assert_eq!(
            alias_name(&tool("ocentra_enforcer_coordination_claim")?)?.as_str(),
            "rust_rules_coordination_claim"
        );
        Ok(())
    }

    #[test]
    fn normalize_tool_name_folds_alias_back_to_canonical() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(
            normalize_tool_name(&tool("rust_rules_check")?)?.as_str(),
            "ocentra_enforcer_check"
        );
        assert_eq!(
            normalize_tool_name(&tool("ocentra_enforcer_check")?)?.as_str(),
            "ocentra_enforcer_check"
        );
        assert_eq!(
            normalize_tool_name(&tool("some_other_tool")?)?.as_str(),
            "some_other_tool"
        );
        Ok(())
    }

    #[test]
    fn every_canonical_tool_round_trips_through_alias_and_back(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for &canonical in CANONICAL_TOOLS {
            let alias = alias_name(&tool(canonical)?)?;
            assert!(alias.as_str().starts_with("rust_rules_"));
            assert_eq!(normalize_tool_name(&alias)?.as_str(), canonical);
        }
        Ok(())
    }

    #[test]
    fn deprecation_window_is_currently_open() {
        // Pass fixture for "alias resolves + appears in tools/list" — see
        // crate::registry::build_tool_descriptors and crate::router tests
        // for the full end-to-end fixtures this flag gates.
        assert!(deprecation_window_open());
    }
}
