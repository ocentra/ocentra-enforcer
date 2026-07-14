//! `CYBER-PROTO-POLLUTION.1` (T1) — harvested from the vendored
//! `exploiting-prototype-pollution-in-javascript` cyberskill
//! (`vendor/anthropic-cybersecurity-skills/skills/exploiting-prototype-pollution-in-javascript`).
//! The vendor skill is mostly a live-target attack playbook (URL/JSON-body
//! payload injection against a running app via `curl`/Burp DOM Invader), but
//! its `scripts/agent.py` also ships one inline static-analysis routine,
//! `scan_source_code()` (L90-115), whose `vulnerable_patterns` table (L93-102)
//! is the concrete harvest target for this validator:
//!
//! - `Object\.assign\s*\([^)]*,\s*\w+\)` — "Object.assign with user input"
//! - `_\.merge\s*\(` — "lodash merge (deep merge)"
//! - `_\.defaultsDeep\s*\(` — "lodash defaultsDeep"
//! - `jQuery\.extend\s*\(\s*true` / `\$\.extend\s*\(\s*true` — "jQuery deep extend"
//! - `JSON\.parse\s*\([^)]*\)` — "JSON.parse (check input source)"
//! - `\.prototype\[` — "Direct prototype access"
//! - `\[(["\'])__proto__\1\]` — "__proto__ string access"
//!
//! The same script's `PROTOTYPE_PAYLOADS`/`PROTOTYPE_PAYLOADS_QUERY` tables
//! (L15-28) confirm the two canonical pollution vectors the skill's "Key
//! Concepts" table names — `__proto__` and `constructor.prototype` — and the
//! gadget properties attackers ride in on (`isAdmin`, `role`, `status`), and
//! the skill's own "Remediation" list (bottom of `SKILL.md`) names the fix
//! this validator enforces: "Sanitize `__proto__` and `constructor` keys in
//! user input", "Use `Object.create(null)`", "Freeze `Object.prototype`".
//!
//! Harvest narrowing (per the PREVENTION-gate false-positive budget): the
//! vendor's bare patterns are tuned for a human-triaged report, not a commit
//! gate. `Object\.assign\s*\([^)]*,\s*\w+\)` matches almost every
//! `Object.assign` call in existence (any bare-identifier second argument);
//! `JSON\.parse\s*\([^)]*\)` matches literally every `JSON.parse` call in a
//! codebase, including ones parsing trusted config. This validator instead
//! requires the merge/assign/extend call to co-occur, on the same line, with
//! a request-derived or `JSON.parse(...)`-derived argument
//! (`req.body`/`req.query`/`req.params`/`JSON.parse(`) before flagging —
//! matching the vendor's own scoping note "(check input source)". It also
//! adds two sinks the vendor names as the pollution vector itself but does
//! not encode as source-scan regexes: a direct write into `obj.__proto__.x`,
//! and the hand-rolled recursive-merge write
//! (`target[key] = source[key]`/`target[key] = merge(target[key],
//! source[key])`) that library calls such as `_.merge` implement internally.
//! Per the vendor's own remediation list, a same-scan-unit denylist guard —
//! `key === "__proto__"` / `key === "constructor"` / `key === "prototype"`
//! (in either comparison order, `===`/`!==`/`==`/`!=`) — is treated as proof
//! the computed-key write is sanitized, and suppresses the merge finding for
//! the whole file; `Object.create(null)`/`Map`/`Set`/`Object.freeze` targets,
//! which the vendor also names as safe alternatives, never match any pattern
//! here to begin with.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One vulnerable deep-merge/extend call-site pattern (agent.py L93-98):
/// a library call whose second/n-th argument is the pollution source, and
/// which only becomes a finding when that argument is request-derived.
struct MergeCallPattern {
    regex: &'static str,
    label: &'static str,
}

/// The vendor's lodash/jQuery/`Object.assign` deep-merge call sites
/// (`scan_source_code`'s `vulnerable_patterns`, agent.py L93-98), each
/// gated below on a same-line untrusted-source argument.
const MERGE_CALL_PATTERNS_SRC: &[MergeCallPattern] = &[
    MergeCallPattern {
        regex: r"_\.merge\s*\(",
        label: "lodash _.merge() (deep merge)",
    },
    MergeCallPattern {
        regex: r"_\.mergeWith\s*\(",
        label: "lodash _.mergeWith() (deep merge with customizer)",
    },
    MergeCallPattern {
        regex: r"_\.defaultsDeep\s*\(",
        label: "lodash _.defaultsDeep()",
    },
    MergeCallPattern {
        regex: r"jQuery\.extend\s*\(\s*true\s*,",
        label: "jQuery.extend(true, ...) deep extend",
    },
    MergeCallPattern {
        regex: r"\$\.extend\s*\(\s*true\s*,",
        label: "$.extend(true, ...) deep extend",
    },
    MergeCallPattern {
        regex: r"Object\.assign\s*\(",
        label: "Object.assign() used as a deep-merge sink",
    },
];

/// `CYBER-PROTO-POLLUTION.1` — flags JavaScript prototype-pollution write
/// sinks: direct `__proto__`/`constructor.prototype` writes, an unguarded
/// hand-rolled computed-key deep-merge, and lodash/jQuery/`Object.assign`
/// deep-merge calls fed a request-derived or `JSON.parse(...)` argument.
pub struct PrototypePollutionValidator {
    rule_id: RuleId,
    proto_dot_write: Regex,
    proto_bracket_literal: Regex,
    constructor_prototype_chain: Regex,
    computed_merge: Regex,
    denylist_guard: Regex,
    merge_calls: Vec<(Regex, &'static str)>,
    untrusted_source: Regex,
}

impl PrototypePollutionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let proto_dot_write = Regex::new(r"\.__proto__\.[\w$]+\s*=(?:[^=]|$)").map_err(|err| {
            DecodeError::new("cyberskillsProtoPollutionDotWrite", err.to_string())
        })?;
        // `obj["__proto__"]`, `obj['constructor']`, `gadget["prototype"]` —
        // agent.py L101's `\[(["\'])__proto__\1\]`, widened to the other two
        // dangerous keys the skill's Key Concepts table names alongside it.
        let proto_bracket_literal =
            Regex::new(r#"\[\s*["'](?:__proto__|constructor|prototype)["']\s*\]"#).map_err(
                |err| DecodeError::new("cyberskillsProtoPollutionBracketLiteral", err.to_string()),
            )?;
        let constructor_prototype_chain =
            Regex::new(r"\.constructor\.prototype\b").map_err(|err| {
                DecodeError::new("cyberskillsProtoPollutionConstructorChain", err.to_string())
            })?;
        // The hand-rolled recursive-merge write every vulnerable deep-merge
        // helper (and the vendor's `_.merge`/`Object.assign` sinks) performs
        // internally: `target[key] = source[key]` or
        // `target[key] = merge(target[key], source[key])`. Deliberately
        // keyed on the literal loop-variable name `key` (the idiomatic
        // `for (const key in source)` spelling) rather than a bare
        // computed-index match, so this does not fire on ordinary indexed
        // copies such as `results[i] = values[i]`.
        let computed_merge =
            Regex::new(r"\b[\w$]+\[\s*key\s*\]\s*=\s*(?:[\w$]+\(\s*)?[\w$]+\[\s*key\s*\]")
                .map_err(|err| {
                    DecodeError::new("cyberskillsProtoPollutionComputedMerge", err.to_string())
                })?;
        // Same-scan-unit denylist guard: a comparison of the loop key
        // against one of the three dangerous property names, in either
        // operand order. Per the vendor's own remediation ("Sanitize
        // __proto__ and constructor keys in user input"), presence of this
        // guard anywhere in the scanned source is treated as proof the
        // computed-key write above is sanitized.
        let denylist_guard = Regex::new(
            r#"(?:===|!==|==|!=)\s*["'](?:__proto__|constructor|prototype)["']|["'](?:__proto__|constructor|prototype)["']\s*(?:===|!==|==|!=)"#,
        )
        .map_err(|err| {
            DecodeError::new("cyberskillsProtoPollutionDenylistGuard", err.to_string())
        })?;
        let mut merge_calls = Vec::with_capacity(MERGE_CALL_PATTERNS_SRC.len());
        for entry in MERGE_CALL_PATTERNS_SRC {
            let regex = Regex::new(entry.regex).map_err(|err| {
                DecodeError::new("cyberskillsProtoPollutionMergeCall", err.to_string())
            })?;
            merge_calls.push((regex, entry.label));
        }
        // Request-derived or JSON.parse(...)-derived argument, per the
        // vendor's own scoping note "JSON.parse (check input source)" and
        // the skill's server-side pollution vectors (`req.body`/query).
        let untrusted_source =
            Regex::new(r"req\.(?:body|query|params)|JSON\.parse\s*\(").map_err(|err| {
                DecodeError::new("cyberskillsProtoPollutionUntrustedSource", err.to_string())
            })?;
        Ok(Self {
            rule_id: "CYBER-PROTO-POLLUTION.1".parse()?,
            proto_dot_write,
            proto_bracket_literal,
            constructor_prototype_chain,
            computed_merge,
            denylist_guard,
            merge_calls,
            untrusted_source,
        })
    }
}

impl Validator for PrototypePollutionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        // Denylist-guard presence is checked over the whole scanned unit,
        // not per-line: the guard is typically an `if` a few lines above
        // the write it protects.
        let has_denylist_guard = self.denylist_guard.is_match(input.source);

        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            if self.proto_dot_write.is_match(line) {
                findings.push(Finding {
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their rule ID.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Direct write to Object.prototype via __proto__".to_owned(),
                    detail: "Line assigns directly into `obj.__proto__.<property>`, writing the \
                             property onto every object's shared prototype rather than the \
                             target object. Fix: never write through `__proto__`; use \
                             `Object.create(null)` for plain data maps and validate/allowlist \
                             property names before any dynamic assignment."
                        .to_owned(),
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their file path.
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            if self.proto_bracket_literal.is_match(line) {
                findings.push(Finding {
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their rule ID.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Computed access to a dangerous prototype key".to_owned(),
                    detail: "Line accesses `__proto__`, `constructor`, or `prototype` through a \
                             computed bracket literal (e.g. `obj[\"__proto__\"]`), the classic \
                             string-keyed pollution vector this skill exploits via \
                             `?__proto__[key]=value` / `{\"__proto__\": {...}}` payloads. Fix: \
                             reject these three key names before any dynamic property write, or \
                             use a `Map` instead of a plain object for user-controlled keys."
                        .to_owned(),
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their file path.
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            if self.constructor_prototype_chain.is_match(line) {
                findings.push(Finding {
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their rule ID.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Prototype access via constructor.prototype".to_owned(),
                    detail: "Line reaches the shared prototype via `X.constructor.prototype`, \
                             the alternative pollution path this skill uses when `__proto__` is \
                             filtered (`{\"constructor\": {\"prototype\": {...}}}`). Fix: never \
                             write through `constructor.prototype`; use `Object.create(null)` \
                             for plain data maps and validate/allowlist property names."
                        .to_owned(),
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their file path.
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            if self.computed_merge.is_match(line) && !has_denylist_guard {
                findings.push(Finding {
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their rule ID.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Unguarded recursive merge write".to_owned(),
                    detail: "Line performs a computed-key deep-merge write \
                             (`target[key] = source[key]`) with no `key === \"__proto__\"` / \
                             `key === \"constructor\"` / `key === \"prototype\"` denylist guard \
                             anywhere in the scanned code, so a source object crafted with a \
                             `__proto__` or `constructor.prototype` key pollutes the shared \
                             Object prototype. Fix: skip the three dangerous key names before \
                             assigning, or build the merge target with `Object.create(null)`."
                        .to_owned(),
                    // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their file path.
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            for (regex, label) in &self.merge_calls {
                if regex.is_match(line) && self.untrusted_source.is_match(line) {
                    findings.push(Finding {
                        // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their rule ID.
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Error,
                        title: "Deep merge/assign fed directly from request input".to_owned(),
                        detail: format!(
                            "Line calls {label} with an argument derived from `req.body` / \
                             `req.query` / `req.params` / `JSON.parse(...)`. Without a \
                             `__proto__`/`constructor`/`prototype` key denylist, this lets an \
                             attacker who controls the request body pollute the shared Object \
                             prototype (e.g. `{{\"__proto__\": {{\"isAdmin\": true}}}}`). Fix: \
                             upgrade to a patched lodash release with key-name filtering, pass \
                             an explicit denylist/allowlist, or merge onto an \
                             `Object.create(null)` target."
                        ),
                        // CLONE-JUSTIFICATION: returned findings outlive the borrowed validation input and own their file path.
                        file: input.file.clone(),
                        line: line_number,
                        snippet: Some(line.to_owned()),
                    });
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::PrototypePollutionValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_proto_pollution() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PrototypePollutionValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.prototype-pollution/bad/vuln.js",
            "tests/fixtures/cyberskills/web.prototype-pollution/good/safe.js",
        )?;
        Ok(())
    }
}
