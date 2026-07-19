//! The `enforcer-security` module-root. Every Track H rule family lives
//! as its own submodule here (this workpack owns [`no_bypass`] and
//! [`registry`] only; feature packs land their own `<name>` submodules
//! alongside these per the workpack's Parallel Ownership Notes).

pub mod boundary;
#[path = "../boundary/rules/economic.rs"]
pub mod economic;
#[path = "../boundary/rules/economic_invariants.rs"]
pub mod economic_invariants;
#[path = "../boundary/rules/killswitch.rs"]
pub mod killswitch;
#[path = "../boundary/rules/money_critical.rs"]
pub mod money_critical;
#[path = "../boundary/rules/no_bypass.rs"]
pub mod no_bypass;
#[path = "../boundary/rules/registry.rs"]
pub mod registry;
pub mod required_test_categories;
#[path = "../boundary/rules/rollback.rs"]
pub mod rollback;
pub mod security_test_quality;
#[path = "../boundary/rules/signing.rs"]
pub mod signing;
#[path = "../boundary/rules/threat_test_mapping.rs"]
pub mod threat_test_mapping;
#[path = "../boundary/rules/time.rs"]
pub mod time;
