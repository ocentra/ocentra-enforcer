//! Non-`PatternValidator` common-family rules that need bespoke logic
//! beyond a literal-marker scan. Sibling to [`crate::families`] (the
//! `PatternValidator`-backed rule tables) and [`crate::port_platform`] (the
//! other existing bespoke validator): each module here owns exactly one
//! `RuleId` plus its fixtures, per the workpack that introduced it.

macro_rules! finding {
    ($rule_id:expr, $severity:expr, $title:expr, $detail:expr, $input:expr, $line:expr $(,)?) => {
        finding!(
            $rule_id,
            $severity,
            $title,
            $detail,
            $input,
            $line,
            crate::boundary::no_snippet(),
        )
    };
    ($rule_id:expr, $severity:expr, $title:expr, $detail:expr, $input:expr, $line:expr, $snippet:expr $(,)?) => {
        match crate::boundary::finding(
            $rule_id,
            $severity,
            ($title, $detail, $snippet),
            ($input).file,
            $line,
        ) {
            Some(finding) => finding,
            None => return Vec::new(),
        }
    };
}

pub mod change_discipline;
pub mod deferred_work;
pub mod fsm;
pub mod resilience;
pub mod size_shape;
pub mod test_quality;
