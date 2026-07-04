//! `typescript/source-scan` — 17 rules (TS-1.1..3, TS-2.1, and a hand-keyed
//! subset of TS-6) whose `rules/rules.json` `triggers` are literal
//! source-level keywords/punctuation (as opposed to the `generic-scanner`
//! TS slice, whose `triggers` field restates the rule title and needs a
//! bespoke pattern per rule — see [`super::generic_scanner`]).

use super::spec::{RuleSpec, TriggerKind};

/// Every `typescript/source-scan` rule's static spec, in `rules.json`
/// declaration order.
pub const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule_id: "TS-1.1",
        title: "TypeScript/JavaScript re-exports are forbidden",
        kind: TriggerKind::Literal,
        needles: &["export * from", "export {", "// barrel", "re-export"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-1.2",
        title: "Direct Zod source usage is forbidden",
        kind: TriggerKind::Word,
        needles: &["zod", "zodResolver", "ZodError"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-1.3",
        title: "Naked domain string aliases are forbidden",
        kind: TriggerKind::Literal,
        needles: &["= string", "__brand"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-2.1",
        title: "TypeScript/JavaScript suppression comments are forbidden",
        kind: TriggerKind::Literal,
        needles: &[
            "eslint-disable",
            "@ts-ignore",
            "@ts-expect-error",
            "@ts-nocheck",
        ],
        // TS-2.1's violation IS a `//`/`/* */` suppression comment — the
        // default comment-only-line skip would defeat this rule entirely.
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "TS-6.1",
        title: "TypeScript any is forbidden",
        kind: TriggerKind::Word,
        needles: &["any"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.10",
        title: "Record<string, domain> APIs are forbidden",
        kind: TriggerKind::Literal,
        needles: &["Record<string, "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.13",
        title: "TypeScript default exports are forbidden",
        kind: TriggerKind::Literal,
        needles: &["export default"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.14",
        title: "Index barrels are forbidden",
        kind: TriggerKind::Literal,
        needles: &["export { ", "export * from"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.18",
        title: "process.env is forbidden outside config boundaries",
        kind: TriggerKind::Literal,
        needles: &["process.env"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.19",
        title: "JSON.parse is forbidden outside decoder boundaries",
        kind: TriggerKind::Literal,
        needles: &["JSON.parse"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.22",
        title: "Floating promises are forbidden",
        kind: TriggerKind::Literal,
        needles: &["saveUser();"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.24",
        title: "console logging is forbidden in source",
        kind: TriggerKind::Literal,
        needles: &[
            "console.log",
            "console.error",
            "console.warn",
            "console.debug",
        ],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.25",
        title: "Throwing string errors is forbidden",
        kind: TriggerKind::Literal,
        needles: &["throw \"", "throw '"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.3",
        title: "TypeScript type assertions are forbidden",
        kind: TriggerKind::Literal,
        needles: &[" as "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.4",
        title: "TypeScript double assertions are forbidden",
        kind: TriggerKind::Literal,
        needles: &["as unknown as"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.5",
        title: "TypeScript non-null assertions are forbidden",
        kind: TriggerKind::NonNullAssertion,
        needles: &[],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.7",
        title: "Raw string domain aliases are forbidden",
        kind: TriggerKind::Literal,
        needles: &["Id = string", "Key = string"],
        comment_guard: true,
    },
];

#[cfg(test)]
mod tests {
    use super::SPECS;
    use crate::rules::spec::SpecValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn every_source_scan_spec_fires_on_fail_and_stays_silent_on_pass(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for spec in SPECS {
            let validator = SpecValidator::new(*spec)?;
            let slug = spec.rule_id.to_lowercase().replace('.', "-");
            let fail = format!("fixtures/source-scan/{slug}/fail.ts");
            let pass = format!("fixtures/source-scan/{slug}/pass.ts");
            run_fixture_parity(&validator, &manifest_dir(), &fail, &pass)
                .map_err(|e| format!("{}: {e}", spec.rule_id))?;
        }
        Ok(())
    }
}
