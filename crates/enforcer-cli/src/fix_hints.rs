//! Terse `Fix:` hints rendered under each finding by [`crate::output`].
//!
//! `enforcer_domain::findings::Finding` carries no hint field today (a
//! validator emits `title`/`detail`, not prose advice) -- this module is
//! the CLI-owned lookup from a rule-id FAMILY prefix (e.g. `RR-6` for
//! `unwrap`/`expect`/`panic` misuse) to one short imperative sentence.
//! Unknown families fall back to a generic-but-still-terse hint rather
//! than silently omitting `Fix:` -- every finding gets one line of
//! actionable text.
//!
//! This is deliberately NOT an override/bypass mechanism: it only adds
//! text to a finding that is still reported and still counts toward the
//! process exit code.

use enforcer_domain::ids::RuleId;

/// One short imperative sentence telling the reader what to change.
pub fn fix_hint(rule_id: &RuleId) -> &'static str {
    let id = rule_id.as_str();
    let family = id.split('.').next().unwrap_or(id);
    match family {
        "RR-6" | "T1-RUSTERR" => "Fix: replace unwrap()/expect()/panic! with a typed Result and `?`.",
        "RR-2" | "T1-NOREEXPORT" => "Fix: remove the `pub use` re-export barrel; import concrete module paths.",
        "SEC-2" | "SEC" => "Fix: remove the hardcoded secret/credential; load it from environment or a vault.",
        "WAIVER-1" => "Fix: add a declarative, committed waiver entry in enforcer-config; no CLI flag can suppress this.",
        "LIT-1" => "Fix: hoist the literal into a named const or config value.",
        _ => "Fix: see the rule detail above; no CLI flag suppresses this finding.",
    }
}

#[cfg(test)]
mod tests {
    use super::fix_hint;
    use enforcer_domain::ids::RuleId;

    fn rule(id: &str) -> Result<RuleId, Box<dyn std::error::Error>> {
        Ok(id.parse()?)
    }

    #[test]
    fn known_family_gets_a_specific_hint() -> Result<(), Box<dyn std::error::Error>> {
        let hint = fix_hint(&rule("RR-6.1")?);
        assert_eq!(
            hint,
            "Fix: replace unwrap()/expect()/panic! with a typed Result and `?`."
        );
        Ok(())
    }

    #[test]
    fn unknown_family_still_gets_a_terse_fallback_hint() -> Result<(), Box<dyn std::error::Error>> {
        let hint = fix_hint(&rule("RR-99.1")?);
        assert_eq!(
            hint,
            "Fix: see the rule detail above; no CLI flag suppresses this finding."
        );
        Ok(())
    }

    #[test]
    fn no_hint_ever_offers_a_bypass() {
        // Static assertion over the whole table: none of the literal hint
        // strings may contain override-flag language.
        let ids = [
            "RR-6.1",
            "RR-2.1",
            "SEC-2.1",
            "WAIVER-1.1",
            "LIT-1.1",
            "XX-1.1",
        ];
        for id in ids {
            let Ok(rule_id) = id.parse::<RuleId>() else {
                continue;
            };
            let hint = fix_hint(&rule_id);
            assert!(!hint.to_lowercase().contains("--force"));
            assert!(!hint.to_lowercase().contains("--skip"));
        }
    }
}
