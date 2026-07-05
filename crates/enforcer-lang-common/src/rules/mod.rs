//! Non-`PatternValidator` common-family rules that need bespoke logic
//! beyond a literal-marker scan. Sibling to [`crate::families`] (the
//! `PatternValidator`-backed rule tables) and [`crate::port_platform`] (the
//! other existing bespoke validator): each module here owns exactly one
//! `RuleId` plus its fixtures, per the workpack that introduced it.

pub mod change_discipline;
pub mod deferred_work;
pub mod fsm;
pub mod size_shape;
