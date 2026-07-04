//! `MONEY-CRIT-CLASSIFY.1` (T2) + `MONEY-CRIT-ANNOTATED.1` (T1) — the
//! money-critical classifier family (h01, §8.2 of the ingested
//! money-critical/security-testing spec).
//!
//! Doctrine (silence != permission; if unsure, treat as money-critical):
//! the enforcer has no mechanical answer today to "is this unit
//! money-critical?" — every downstream Track H rule (h02 required test
//! categories, h03 threat/invariant mapping, h05 economic-invariant suite)
//! needs one. This module supplies it, GENERICALLY across any value system
//! (fiat, Stripe, AWS-billed metering, an internal ledger, or the optional
//! crypto/Anchor instance) — never crypto-only.
//!
//! # Two validators, one pipeline
//!
//! - [`MoneyCriticalClassifyValidator`] (`MONEY-CRIT-CLASSIFY.1`, T2): a
//!   `syn`-AST-driven scored classifier. Every free function / inherent-impl
//!   method / trait-impl method is scored against the enumerated
//!   value-touching signal families from §8.2 — balance/credit/reward/
//!   cooldown mutation, transfer/mint/burn, economic calculation, payment
//!   signing/authorization, rollback/compensation, time-based state change,
//!   kill-switch toggling. Crossing the threshold classifies the unit as
//!   money-critical (`score`+`confidence` are carried in the finding
//!   detail, not a separate wire field — `Finding` has no scored-model slot
//!   yet; see the module-level score/confidence helpers for the machine-
//!   readable shape a future consumer parses from `detail`).
//! - [`MoneyCriticalAnnotatedValidator`] (`MONEY-CRIT-ANNOTATED.1`, T1): runs
//!   the SAME classifier, then gates — a classified unit MUST carry the
//!   explicit `#[money_critical(registered)]` annotation (the manifest
//!   entry, expressed as an attribute at the point of definition rather
//!   than a separate out-of-band file, so registration cannot silently
//!   drift from the code it registers). A classified-but-unannotated unit
//!   is flagged. This is also where "if unsure, treat as money-critical"
//!   lives: an AMBIGUOUS unit (scored, but only just crossing threshold) is
//!   treated identically to a confidently-classified one for gating
//!   purposes — there is no lower confidence tier that gets a pass. The
//!   only way out is the explicit annotation.
//!
//! `#[money_critical(exempt)]` is the one recognized way to mark a
//! classified-looking unit as deliberately NOT money-critical (e.g. a test
//! helper named `credit_balance_for_fixture` that never touches production
//! state) — still an explicit, auditable annotation, never silent.
//!
//! # Non-goals
//!
//! This module does not itself maintain a separate manifest FILE; the
//! attribute IS the manifest entry, colocated with the code it describes
//! (grep-able, cannot drift from a moved/renamed function the way an
//! external path-keyed list would). h02/h03/h05/h06 consume this
//! classification (via the same `syn`-AST signal detection, or by reading
//! `#[money_critical(..)]` attributes emitted by this crate) read-only —
//! they must not redefine what counts as money-critical.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ImplItemFn, ItemFn, Signature, TraitItemFn};

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// One value-touching signal family from §8.2, paired with the lexical
/// markers (matched case-insensitively against the function's identifier
/// AND its body's textual tokens) that indicate it. Family names double as
/// the human-readable label in finding details.
struct SignalFamily {
    name: &'static str,
    markers: &'static [&'static str],
    weight: i32,
}

/// The enumerated §8.2 signal families. GENERIC across any value system —
/// no crypto-only marker set (`lamports`/`anchor` sit alongside `stripe`/
/// `ledger`/`balance` as one family, never their own privileged track).
const SIGNAL_FAMILIES: &[SignalFamily] = &[
    SignalFamily {
        name: "creates/transfers/modifies/destroys value",
        markers: &[
            "balance", "credit", "debit", "transfer", "mint", "burn", "deposit", "withdraw",
            "ledger", "wallet", "stripe", "lamports", "invoice", "refund",
        ],
        weight: 40,
    },
    SignalFamily {
        name: "performs economic calculation",
        markers: &[
            "price",
            "pricing",
            "fee",
            "interest",
            "exchange_rate",
            "exchangerate",
            "discount",
            "tax",
            "payout",
            "settlement",
        ],
        weight: 35,
    },
    SignalFamily {
        name: "applies rewards/credits/balances/cooldowns",
        markers: &["reward", "bonus", "cooldown", "loyalty", "cashback"],
        weight: 35,
    },
    SignalFamily {
        name: "signs or authorizes payments",
        markers: &[
            "sign_payment",
            "authorize_payment",
            "signpayment",
            "payment_signature",
        ],
        weight: 45,
    },
    SignalFamily {
        name: "executes rollback/compensation",
        markers: &[
            "rollback",
            "compensate",
            "compensation",
            "reverse_transaction",
        ],
        weight: 35,
    },
    SignalFamily {
        name: "changes time-based state",
        markers: &[
            "expire",
            "expiry",
            "accrue",
            "schedule_payout",
            "vesting",
            "unlock_at",
        ],
        weight: 25,
    },
    SignalFamily {
        name: "toggles kill-switches",
        markers: &[
            "kill_switch",
            "killswitch",
            "circuit_breaker",
            "emergency_halt",
            "pause_payments",
        ],
        weight: 30,
    },
];

/// Score >= this threshold classifies the unit as money-critical.
const CLASSIFY_THRESHOLD: i32 = 40;

/// One scored unit: identifier, score, and which families fired — enough
/// for a caller to render a human-readable finding or (for h02/h03/h05) to
/// re-derive the classification decision deterministically.
struct ScoredUnit<'a> {
    ident: String,
    score: i32,
    fired: Vec<&'a str>,
}

impl ScoredUnit<'_> {
    fn is_classified(&self) -> bool {
        self.score >= CLASSIFY_THRESHOLD
    }

    /// Coarse confidence bucket derived from the score, purely for the
    /// human-readable finding detail (doctrine: ambiguous-but-classified
    /// still gates identically to confidently-classified — this label is
    /// informational, never a gating input).
    fn confidence_label(&self) -> &'static str {
        if self.score >= CLASSIFY_THRESHOLD.saturating_mul(2) {
            "high"
        } else if self.score >= CLASSIFY_THRESHOLD {
            "ambiguous (unsure -> treated as money-critical)"
        } else {
            "low"
        }
    }
}

/// Score one function-like unit's identifier + body source text against
/// every [`SIGNAL_FAMILIES`] entry.
fn score_unit(ident: &str, body_text: &str) -> ScoredUnit<'static> {
    let haystack_ident = ident.to_ascii_lowercase();
    let haystack_body = body_text.to_ascii_lowercase();
    let mut score = 0i32;
    let mut fired = Vec::new();
    for family in SIGNAL_FAMILIES {
        let hit = family
            .markers
            .iter()
            .any(|marker| haystack_ident.contains(marker) || haystack_body.contains(marker));
        if hit {
            score += family.weight;
            fired.push(family.name);
        }
    }
    ScoredUnit {
        ident: ident.to_owned(),
        score,
        fired,
    }
}

/// True when `attrs` carries `#[money_critical(registered)]`.
fn is_registered(attrs: &[Attribute]) -> bool {
    has_money_critical_arg(attrs, "registered")
}

/// True when `attrs` carries `#[money_critical(exempt)]` — an explicit,
/// auditable opt-out for a unit that scores as value-touching but is
/// deliberately not money-critical (e.g. a fixture/test helper).
fn is_exempt(attrs: &[Attribute]) -> bool {
    has_money_critical_arg(attrs, "exempt")
}

fn has_money_critical_arg(attrs: &[Attribute], want: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("money_critical") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(want) {
                found = true;
            }
            Ok(())
        });
        found
    })
}

/// One function-like item collected during the AST walk, carrying enough
/// to score, gate, and locate it.
struct CollectedFn {
    ident: String,
    attrs: Vec<Attribute>,
    body_text: String,
    line: u32,
}

fn line_of<S: Spanned>(spanned: &S) -> u32 {
    let line = spanned.span().start().line;
    if line == 0 {
        1
    } else {
        u32::try_from(line).unwrap_or(u32::MAX)
    }
}

fn sig_ident(sig: &Signature) -> String {
    sig.ident.to_string()
}

/// Render a function-like item's body as plain text for lexical marker
/// scanning. `syn` AST nodes built with the `extra-traits` feature derive
/// `Debug`, whose output includes every sub-token's textual content
/// (identifiers, literals) — sufficient for the case-insensitive substring
/// marker matching [`score_unit`] does with it, without pulling in a
/// separate `quote`/pretty-printer dependency this crate otherwise has no
/// need for.
fn body_text_of<T: std::fmt::Debug>(item: &T) -> String {
    format!("{item:?}")
}

struct FnCollector {
    functions: Vec<CollectedFn>,
}

impl<'ast> Visit<'ast> for FnCollector {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.functions.push(CollectedFn {
            ident: sig_ident(&item.sig),
            attrs: item.attrs.clone(),
            body_text: body_text_of(item),
            line: line_of(item),
        });
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.functions.push(CollectedFn {
            ident: sig_ident(&item.sig),
            attrs: item.attrs.clone(),
            body_text: body_text_of(item),
            line: line_of(item),
        });
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        self.functions.push(CollectedFn {
            ident: sig_ident(&item.sig),
            attrs: item.attrs.clone(),
            body_text: body_text_of(item),
            line: line_of(item),
        });
        visit::visit_trait_item_fn(self, item);
    }
}

fn collect_functions(file: &syn::File) -> Vec<CollectedFn> {
    let mut collector = FnCollector {
        functions: Vec::new(),
    };
    collector.visit_file(file);
    collector.functions
}

/// `MONEY-CRIT-CLASSIFY.1` — T2 scored classifier.
///
/// Fires on any function-like unit whose score crosses
/// [`CLASSIFY_THRESHOLD`], regardless of annotation state (the annotation
/// gate is [`MoneyCriticalAnnotatedValidator`]'s job) — this validator's
/// sole contract is "does this look money-critical", proven by a fail
/// fixture that crosses threshold and a pass fixture (a pure formatter)
/// that stays under it.
pub struct MoneyCriticalClassifyValidator {
    rule_id: RuleId,
}

impl MoneyCriticalClassifyValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MONEY-CRIT-CLASSIFY.1".parse()?,
        })
    }
}

impl Validator for MoneyCriticalClassifyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            // Unparseable source is not this validator's concern.
            return Vec::new();
        };

        let mut findings = Vec::new();
        for func in collect_functions(&file) {
            let scored = score_unit(&func.ident, &func.body_text);
            if !scored.is_classified() {
                continue;
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Warning,
                title: "unit classified money-critical (T2 scored)".to_owned(),
                detail: format!(
                    "`{}` scores {} (threshold {CLASSIFY_THRESHOLD}, confidence: {}) on \
                     value-touching signal families: {}. Fix: if this unit truly is \
                     money-critical, register it with `#[money_critical(registered)]`; if it \
                     is a false positive, mark it `#[money_critical(exempt)]` — silence is not \
                     permission, an unannotated classified unit will be flagged by \
                     MONEY-CRIT-ANNOTATED.1.",
                    scored.ident,
                    scored.score,
                    scored.confidence_label(),
                    scored.fired.join(", ")
                ),
                file: input.file.clone(),
                line: func.line,
                snippet: None,
            });
        }
        findings
    }
}

/// `MONEY-CRIT-ANNOTATED.1` — T1 annotation/registration gate.
///
/// Re-runs the same classification signal (never redefines it — doctrine:
/// downstream consumers must not redefine classification, and this
/// validator IS one such consumer of its sibling's logic) and requires
/// every classified unit to carry `#[money_critical(registered)]`.
/// Doctrine: "if unsure, treat as money-critical" — an ambiguous
/// value-adjacent unit that merely crosses threshold gates identically to
/// a confidently-classified one; the only escape is the explicit
/// `#[money_critical(exempt)]` annotation.
pub struct MoneyCriticalAnnotatedValidator {
    rule_id: RuleId,
}

impl MoneyCriticalAnnotatedValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "MONEY-CRIT-ANNOTATED.1".parse()?,
        })
    }
}

impl Validator for MoneyCriticalAnnotatedValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        for func in collect_functions(&file) {
            let scored = score_unit(&func.ident, &func.body_text);
            if !scored.is_classified() {
                continue;
            }
            if is_exempt(&func.attrs) {
                continue;
            }
            if is_registered(&func.attrs) {
                continue;
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "classified money-critical unit is unannotated (T1 gate)".to_owned(),
                detail: format!(
                    "`{}` is classified money-critical (score {}, signals: {}) but carries \
                     neither `#[money_critical(registered)]` nor `#[money_critical(exempt)]`. \
                     Doctrine: silence is not permission — if unsure, treat as money-critical. \
                     Fix: add `#[money_critical(registered)]` above this item (or \
                     `#[money_critical(exempt)]` if this is a confirmed false positive).",
                    scored.ident,
                    scored.score,
                    scored.fired.join(", ")
                ),
                file: input.file.clone(),
                line: func.line,
                snippet: None,
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_validator::harness::run_fixture_parity;

    use super::{MoneyCriticalAnnotatedValidator, MoneyCriticalClassifyValidator};
    use enforcer_validator::validator::{ValidationInput, Validator};

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    use enforcer_domain::paths::RelPath;

    #[test]
    fn money_crit_classify_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MoneyCriticalClassifyValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/money_critical/classify/bad/credit_balance.rs",
            "tests/fixtures/money_critical/classify/good/pure_formatter.rs",
        )?;
        Ok(())
    }

    #[test]
    fn money_crit_annotated_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MoneyCriticalAnnotatedValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/money_critical/annotated/bad/sign_payment_unannotated.rs",
            "tests/fixtures/money_critical/annotated/good/sign_payment_registered.rs",
        )?;
        Ok(())
    }

    #[test]
    fn classify_annotated_pair_stays_clean_under_annotated_gate(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Representative triple from the workpack: a classified+annotated
        // unit must NOT be flagged by the T1 gate (only the T2 classifier
        // still notes it as classified; annotation state is orthogonal to
        // classification itself).
        let validator = MoneyCriticalAnnotatedValidator::new()?;
        let file = rel("crates/x/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "#[money_critical(registered)]\nfn credit_balance(id: u64, amount: u64) { let _ = (id, amount); }\n",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn exempt_annotation_silences_a_classified_unit() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MoneyCriticalAnnotatedValidator::new()?;
        let file = rel("crates/x/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "#[money_critical(exempt)]\nfn credit_balance_for_fixture() {}\n",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn ambiguous_unit_still_gates_like_a_confident_classification(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Doctrine: "if unsure, treat as money-critical" — a unit that only
        // just crosses threshold (single weak signal family) must still be
        // flagged when unannotated, not given a pass for low confidence.
        let validator = MoneyCriticalAnnotatedValidator::new()?;
        let file = rel("crates/x/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "fn schedule_payout(id: u64) { let _ = id; }\n",
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings[0]
            .detail
            .contains("if unsure, treat as money-critical"));
        Ok(())
    }

    #[test]
    fn pure_formatter_is_never_classified() -> Result<(), Box<dyn std::error::Error>> {
        let classify = MoneyCriticalClassifyValidator::new()?;
        let file = rel("crates/x/src/lib.rs")?;
        let findings = classify.validate(ValidationInput {
            file: &file,
            source: "fn format_greeting(name: &str) -> String { format!(\"hello {name}\") }\n",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn credit_balance_triple_classified_and_annotated_is_clean(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Workpack "representative triple": the balance-crediting fixture,
        // registered, must still classify (T2 fires) but stay clean under
        // the T1 annotation gate.
        let source = std::fs::read_to_string(
            manifest_dir()
                .join("tests/fixtures/money_critical/classify/good/credit_balance_annotated.rs"),
        )?;
        let file = rel("crates/x/src/lib.rs")?;

        let classify = MoneyCriticalClassifyValidator::new()?;
        let classify_findings = classify.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(
            !classify_findings.is_empty(),
            "annotation state must not suppress T2 classification"
        );

        let annotated = MoneyCriticalAnnotatedValidator::new()?;
        let annotated_findings = annotated.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(
            annotated_findings.is_empty(),
            "registered units must stay clean under the T1 gate"
        );
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent_for_both_validators(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = rel("crates/x/src/lib.rs")?;
        let classify = MoneyCriticalClassifyValidator::new()?;
        let annotated = MoneyCriticalAnnotatedValidator::new()?;
        let source = "this is not valid rust {{{";
        assert!(classify
            .validate(ValidationInput {
                file: &file,
                source,
                scope: ScanScope::Files,
            })
            .is_empty());
        assert!(annotated
            .validate(ValidationInput {
                file: &file,
                source,
                scope: ScanScope::Files,
            })
            .is_empty());
        Ok(())
    }
}
