//! Module root for `enforcer-scan`'s hosted rule-adjacent machinery.
//!
//! arc-15 owns this module root and hosts [`baseline_ratchet`] (d02) here
//! as a skeleton. Sibling scan-rule packs that add scan-specific rules own
//! SPECIFIC sibling files under this directory (`src/rules/<name>.rs` +
//! `tests/fixtures/<name>/**`), `deps: arc-15` — this file declares
//! submodules, it does not re-export their contents.

pub mod baseline_ratchet;
