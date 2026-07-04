//! One module per common-family `RuleId` prefix (see the workpack's
//! "Rule inventory (per-prefix)" table). Each module's `validators()`
//! builds that family's `PatternValidator`s; `crate::registry::all()`
//! concatenates every family plus the bespoke `PORT-1.1` validator.

pub mod ai_1;
pub mod arch_1;
pub mod bound_1;
pub mod cfg_1;
pub mod ci_1;
pub mod contract_1;
pub mod dep_1;
pub mod doc_1;
pub mod docenf_1;
pub mod enf_1;
pub mod enf_2;
pub mod gen_1;
pub mod gen_2;
pub mod har_1;
pub mod har_2;
pub mod lit_1;
pub mod mcp_1;
pub mod npm_1;
pub mod proof_1;
pub mod repo_1;
pub mod sbom_1;
pub mod scan_1;
pub mod scan_2;
pub mod sec_1;
pub mod src_1;
pub mod src_2;
pub mod test_1;
pub mod test_2;
pub mod waiver_1;
