//! CP00 truth-ledger tests for CyberSkills identity and decomposition.

use enforcer_rules::cyberskills_disposition::{
    parse_manifest, validate_manifest, PROTECTED_CATALOG_ID, PROTECTED_SOURCE_PATH,
    PROTECTED_TRACKED_BLOB,
};

const DISPOSITION_JSON: &str = include_str!("../dispositions/cyberskills-disposition.json");
const NATIVE_RULES_JSON: &str = include_str!("../rules/cyberskills.json");
const ADAPTER_RULES_JSON: &str = include_str!("../rules/cyberskills-adapters.json");
const SOURCE_CATALOG: &str = include_str!(
    "../../../docs/plans/enforcer-selfhost-plan/refs/cyberskills-mechanization-catalog.md"
);
const NEGATIVE_FIXTURES: &str =
    include_str!("fixtures/cyberskills_disposition/negative_cases.json");

#[path = "cyberskills_disposition/cp08.rs"]
mod cp08;
#[path = "cyberskills_disposition/cp08_validation.rs"]
mod cp08_validation;
#[path = "cyberskills_disposition/manifest.rs"]
mod manifest;
#[path = "cyberskills_disposition/negative.rs"]
mod negative;
#[path = "cyberskills_disposition/negative_mutations.rs"]
mod negative_mutations;
#[path = "cyberskills_disposition/support.rs"]
mod support;
