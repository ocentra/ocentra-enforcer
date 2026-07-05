//! Per-target run outcome — the anti-silent-skip primitive.
//!
//! The legacy generic scanners and CLI scan could early-return on an
//! unmatched extension, a missing tool, or an empty selection with no
//! emitted record at all: a validator that ran on nothing looked
//! identical, downstream, to one that ran and passed. [`Outcome`] closes
//! that gap by making every dispatch decision an explicit, non-erasable
//! value: either the target [`Outcome::Ran`] (with the validators that
//! actually executed against it) or it was [`Outcome::Skipped`] for a
//! caller-supplied, guaranteed-non-empty `reason`. There is no third,
//! silent option — every candidate handed to the engine must produce one
//! of these two variants.

use enforcer_core::error::DecodeError;

/// A non-empty skip reason. Parsed at the boundary: constructing a
/// [`SkipReason`] from an empty or all-whitespace string fails, so
/// `Outcome::Skipped` can never carry a reason that would render as blank
/// in a report (which would be indistinguishable from a silent skip).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(try_from = "String", into = "String")]
pub struct SkipReason(String);

impl SkipReason {
    /// Borrow the reason text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SkipReason {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            Err(DecodeError::new(
                "skip_reason",
                "a skip reason must not be empty — a skip with no reason is a silent skip",
            ))
        } else {
            Ok(Self(value))
        }
    }
}

impl From<SkipReason> for String {
    fn from(reason: SkipReason) -> String {
        reason.0
    }
}

impl std::str::FromStr for SkipReason {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, DecodeError> {
        Self::try_from(s.to_owned())
    }
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What happened to one candidate target during a scan dispatch.
///
/// Every target handed to the engine's per-file dispatch loop must
/// resolve to exactly one of these — there is no code path that drops a
/// target without recording either `Ran` or `Skipped`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum Outcome {
    /// The target was dispatched to at least a router decision and
    /// validators actually ran against it (`validator_count` may be zero
    /// only when the family legitimately has no applicable validators —
    /// callers that want anti-silent-skip at the aggregate level should
    /// still check [`crate::coverage::Coverage`], which hard-fails when
    /// the total ran-count across the whole scan is zero).
    Ran {
        /// How many validators actually executed against this target.
        validator_count: usize,
    },
    /// The target was not run, for an explicit, non-empty reason (e.g.
    /// unmatched extension, missing tool, empty selection, unreadable
    /// file).
    Skipped {
        /// Why this target did not run. Never empty — see [`SkipReason`].
        reason: SkipReason,
    },
}

impl Outcome {
    /// Construct a [`Outcome::Ran`] outcome.
    pub fn ran(validator_count: usize) -> Self {
        Self::Ran { validator_count }
    }

    /// Construct a [`Outcome::Skipped`] outcome from a reason string.
    ///
    /// # Errors
    /// Returns [`DecodeError`] if `reason` is empty or all-whitespace.
    pub fn skipped(reason: impl Into<String>) -> Result<Self, DecodeError> {
        Ok(Self::Skipped {
            reason: SkipReason::try_from(reason.into())?,
        })
    }

    /// True if this target actually ran (regardless of validator count).
    pub fn did_run(&self) -> bool {
        matches!(self, Self::Ran { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::{Outcome, SkipReason};

    #[test]
    fn skip_reason_rejects_empty() {
        assert!(SkipReason::try_from(String::new()).is_err());
        assert!(SkipReason::try_from("   ".to_owned()).is_err());
    }

    #[test]
    fn skip_reason_accepts_non_empty() -> Result<(), Box<dyn std::error::Error>> {
        let reason = SkipReason::try_from("unmatched extension".to_owned())?;
        assert_eq!(reason.as_str(), "unmatched extension");
        Ok(())
    }

    #[test]
    fn outcome_skipped_rejects_empty_reason() {
        assert!(Outcome::skipped("").is_err());
        assert!(Outcome::skipped("   ").is_err());
    }

    #[test]
    fn outcome_skipped_accepts_reason() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = Outcome::skipped("missing tool: cargo-clippy")?;
        assert!(!outcome.did_run());
        Ok(())
    }

    #[test]
    fn outcome_ran_reports_did_run() {
        let outcome = Outcome::ran(3);
        assert!(outcome.did_run());
    }

    #[test]
    fn outcome_wire_form_is_camel_case_tagged() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = Outcome::skipped("unmatched extension")?;
        let wire = serde_json::to_value(&outcome)?;
        assert_eq!(wire["kind"], "skipped");
        assert_eq!(wire["reason"], "unmatched extension");

        let ran = Outcome::ran(2);
        let wire_ran = serde_json::to_value(&ran)?;
        assert_eq!(wire_ran["kind"], "ran");
        assert_eq!(wire_ran["validatorCount"], 2);
        Ok(())
    }

    #[test]
    fn outcome_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = Outcome::skipped("empty selection")?;
        let wire = serde_json::to_string(&outcome)?;
        let back: Outcome = serde_json::from_str(&wire)?;
        assert_eq!(back, outcome);
        Ok(())
    }

    #[test]
    fn outcome_rejects_smuggled_empty_reason_on_decode() -> Result<(), Box<dyn std::error::Error>>
    {
        let outcome = Outcome::skipped("real reason")?;
        let wire = serde_json::to_string(&outcome)?;
        let smuggled = wire.replace("real reason", "");
        assert!(serde_json::from_str::<Outcome>(&smuggled).is_err());
        Ok(())
    }
}
