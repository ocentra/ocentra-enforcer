//! Bespoke (non-line-marker) rule modules for this crate, sibling to the
//! `rules.json`-backed [`crate::source_scan`]/[`crate::test_scan`]/etc.
//! modules. Each module here owns its own disjoint `RuleId` family plus
//! fixtures, per the workpack that introduced it — none of these ids
//! appear in `rules/rules.json`'s `language == "python"` set, so they are
//! NOT part of this crate's 61-count registry-coverage assertion.

pub mod fastapi_layered;
