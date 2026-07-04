//! The TypeScript SLICE of the shared `generic-scanner` engine (48 rules:
//! TS-6.2/6/8/9/11/12/15..40 minus the source-scan subset, TS-7.2..11/14/15,
//! TS-8.1..9). arc-07 owns only this TS slice's rule specs, NOT the shared
//! `generic-scanner` engine itself — the engine and its
//! common/python/typescript partition are owned by arc-09 (see this
//! workpack's `SHARED-ENGINE NOTE`).
//!
//! `rules/rules.json`'s `triggers` field for every rule in this slice
//! restates the rule TITLE rather than giving a literal source keyword (an
//! artifact of how the JSON catalog was authored for prose-level triggers
//! vs. the `source-scan` family's literal-keyword triggers) — each spec
//! below therefore encodes a bespoke, semantically-derived pattern from the
//! rule's `title`/`snippet`, not a copy of the JSON `triggers` string.

use super::spec::{RuleSpec, TriggerKind};

/// Every TS-slice `generic-scanner` rule's static spec, grouped by
/// `rules.json` family (`TS-6` type-safety source rules, `TS-7` toolchain
/// config rules, `TS-8` test-quality rules).
pub const SPECS: &[RuleSpec] = &[
    // --- TS-6 type-safety / domain-shape rules ---------------------------
    RuleSpec {
        rule_id: "TS-6.2",
        title: "TypeScript unknown cannot escape boundaries",
        kind: TriggerKind::Literal,
        needles: &[
            "): unknown {",
            "export function decode(raw: string): unknown",
        ],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.6",
        title: "TypeScript definite assignment assertions are forbidden",
        kind: TriggerKind::Literal,
        needles: &["!: "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.8",
        title: "Raw number domain values are forbidden",
        kind: TriggerKind::Literal,
        needles: &["quantity: number", "count: number", "amount: number"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.9",
        title: "Raw boolean domain parameters are forbidden",
        kind: TriggerKind::Literal,
        needles: &["enabled: boolean", "isActive: boolean", "flag: boolean"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.11",
        title: "Map<string, domain> APIs are forbidden",
        kind: TriggerKind::Literal,
        needles: &["Map<string, "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.12",
        title: "String arrays are forbidden in domain APIs",
        kind: TriggerKind::Literal,
        needles: &["string[]"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.15",
        title: "TypeScript namespace declarations are forbidden",
        kind: TriggerKind::Literal,
        needles: &["namespace "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.16",
        title: "TypeScript enums are forbidden by default",
        kind: TriggerKind::Literal,
        needles: &["enum "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.17",
        title: "Ambient declare global is forbidden outside type owners",
        kind: TriggerKind::Literal,
        needles: &["declare global"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.20",
        title: "Date is forbidden in domain APIs",
        kind: TriggerKind::Literal,
        needles: &[": Date", "new Date("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.21",
        title: "Promise<any> and Promise<unknown> are forbidden",
        kind: TriggerKind::Literal,
        needles: &["Promise<any>", "Promise<unknown>"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.23",
        title: "Swallowed promise catches are forbidden",
        kind: TriggerKind::Literal,
        needles: &[".catch(() => {})", ".catch(() => {});"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.26",
        title: "return null is forbidden in domain APIs",
        kind: TriggerKind::Literal,
        needles: &["return null"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.27",
        title: "undefined domain state is forbidden",
        kind: TriggerKind::Literal,
        needles: &["return undefined", "= undefined;"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.28",
        title: "Optional domain fields are forbidden by default",
        kind: TriggerKind::Literal,
        needles: &["?: "],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.29",
        title: "Partial<T> is forbidden in domain logic",
        kind: TriggerKind::Literal,
        needles: &["Partial<"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.30",
        title: "Record<string, unknown> payloads are forbidden",
        kind: TriggerKind::Literal,
        needles: &["Record<string, unknown>"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.31",
        title: "Timer sleeps are forbidden by default",
        kind: TriggerKind::Literal,
        needles: &["setTimeout("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.32",
        title: "Dynamic imports are forbidden in domain code",
        kind: TriggerKind::Literal,
        needles: &["import("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.33",
        title: "child_process is forbidden outside script boundaries",
        kind: TriggerKind::Literal,
        needles: &["child_process"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.34",
        title: "Dynamic code execution is forbidden",
        kind: TriggerKind::Literal,
        needles: &["eval(", "new Function("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.35",
        title: "Spreading raw DTOs into domain objects is forbidden",
        kind: TriggerKind::Literal,
        needles: &["...dto", "...raw", "...payload", "...json"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.36",
        title: "Spreading any into domain objects is forbidden",
        kind: TriggerKind::Literal,
        needles: &["...(raw as any)", "...(input as any)"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.37",
        title: "Exported functions require explicit return types",
        kind: TriggerKind::Literal,
        needles: &["export function widget() {", "export function process() {"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.38",
        title: "Exported object literals cannot be inferred APIs",
        kind: TriggerKind::Literal,
        needles: &["export const config = {", "export const settings = {"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.39",
        title: "Use const instead of single-assignment let",
        kind: TriggerKind::Word,
        needles: &["let"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-6.40",
        title: "Mutating imported or shared objects is forbidden",
        kind: TriggerKind::Literal,
        needles: &["shared.", "config.value ="],
        comment_guard: true,
    },
    // --- TS-7 toolchain config rules ---------------------------------------
    RuleSpec {
        rule_id: "TS-7.2",
        title: "noImplicitAny is required",
        kind: TriggerKind::Literal,
        needles: &["\"noImplicitAny\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.3",
        title: "strictNullChecks is required",
        kind: TriggerKind::Literal,
        needles: &["\"strictNullChecks\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.4",
        title: "noUncheckedIndexedAccess is required",
        kind: TriggerKind::Literal,
        needles: &["\"noUncheckedIndexedAccess\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.5",
        title: "exactOptionalPropertyTypes is required",
        kind: TriggerKind::Literal,
        needles: &["\"exactOptionalPropertyTypes\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.6",
        title: "noImplicitOverride is required",
        kind: TriggerKind::Literal,
        needles: &["\"noImplicitOverride\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.7",
        title: "noPropertyAccessFromIndexSignature is required",
        kind: TriggerKind::Literal,
        needles: &["\"noPropertyAccessFromIndexSignature\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.8",
        title: "useUnknownInCatchVariables is required",
        kind: TriggerKind::Literal,
        needles: &["\"useUnknownInCatchVariables\": false"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.9",
        title: "skipLibCheck policy must be explicit",
        kind: TriggerKind::Literal,
        needles: &["\"skipLibCheck\": undefined"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.10",
        title: "Package manager lockfile is required",
        kind: TriggerKind::Literal,
        needles: &["\"lockfile\": \"none\""],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.11",
        title: "Loose npm dependency versions are forbidden",
        kind: TriggerKind::Literal,
        needles: &["\"^", "\"~", "\"*\"", "\"latest\""],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.14",
        title: "Zod dependencies are forbidden by policy",
        kind: TriggerKind::Literal,
        needles: &["\"zod\":"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-7.15",
        title: "Duplicate package managers are forbidden by default",
        kind: TriggerKind::Literal,
        needles: &["yarn.lock", "pnpm-lock.yaml"],
        comment_guard: true,
    },
    // --- TS-8 test-quality rules --------------------------------------------
    RuleSpec {
        rule_id: "TS-8.1",
        title: "Skipped and focused TypeScript tests are forbidden",
        kind: TriggerKind::Literal,
        needles: &["it.skip(", "it.only(", "test.todo("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.2",
        title: "TypeScript expect.any assertions are forbidden",
        kind: TriggerKind::Literal,
        needles: &["expect.any(", "expect.anything("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.3",
        title: "Weak TypeScript assertions are forbidden",
        kind: TriggerKind::Literal,
        needles: &["toBeTruthy()", "toBeDefined()", "not.toThrow()"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.4",
        title: "Empty TypeScript tests are forbidden",
        kind: TriggerKind::Literal,
        needles: &["() => {});"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.5",
        title: "TypeScript tests must assert behavior",
        kind: TriggerKind::Literal,
        needles: &["// no assertion"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.6",
        title: "Network calls are forbidden in TypeScript unit tests",
        kind: TriggerKind::Literal,
        needles: &["fetch(\"http", "axios.get(\"http"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.7",
        title: "Real timers are forbidden in deterministic TypeScript tests",
        kind: TriggerKind::Literal,
        needles: &["await sleep(", "setTimeout(resolve,"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.8",
        title: "TypeScript test doubles are forbidden by default",
        kind: TriggerKind::Literal,
        needles: &["jest.mock(", "sinon.stub(", "jest.spyOn("],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "TS-8.9",
        title: "Snapshots cannot contain volatile values",
        kind: TriggerKind::Literal,
        needles: &["toMatchSnapshot()"],
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
    fn every_generic_scanner_ts_slice_spec_fires_on_fail_and_stays_silent_on_pass(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for spec in SPECS {
            let validator = SpecValidator::new(*spec)?;
            let slug = spec.rule_id.to_lowercase().replace('.', "-");
            let fail = format!("fixtures/generic-scanner/{slug}/fail.ts");
            let pass = format!("fixtures/generic-scanner/{slug}/pass.ts");
            run_fixture_parity(&validator, &manifest_dir(), &fail, &pass)
                .map_err(|e| format!("{}: {e}", spec.rule_id))?;
        }
        Ok(())
    }
}
