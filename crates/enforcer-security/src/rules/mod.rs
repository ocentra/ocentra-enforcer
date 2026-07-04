//! The `enforcer-security` module-root. Every Track H rule family lives
//! as its own submodule here (this workpack owns [`no_bypass`] and
//! [`registry`] only; feature packs land their own `<name>` submodules
//! alongside these per the workpack's Parallel Ownership Notes).

pub mod money_critical;
pub mod no_bypass;
pub mod registry;
pub mod threat_test_mapping;
