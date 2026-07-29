//! Canonical global profiles, embedded into the binary at compile time so
//! the engine is self-contained ("one binary IS the engine" doctrine) — no
//! external `profiles/` directory is required for baseline operation. A
//! project may still supply additional custom profiles as external files
//! (not modeled here: `crate::resolve` accepts a raw JSON string for those).

/// Raw embedded-profile wire names, private to the config transport boundary.
pub(crate) const KNOWN_PROFILE_NAMES: [&str; 4] =
    ["strict", "default", "ocentra-enforcer", "ocentra-parent"];
