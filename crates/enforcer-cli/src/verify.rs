//! The `verify --mode {fast,local,ci,parent}` axis (WAVE 4 port of the
//! Node `VERIFY_MODE_CHECKS`/`normalizeVerifyMode`).
//!
//! `verify` is a scope/aggregation PROFILE over checks, orthogonal to the
//! d06 lifecycle `plan|implement|check|fix|review` family -- see
//! `crate::cli` module docs. `--mode` defaults to `local`; an empty
//! string also normalizes to `local` (matching the legacy coercion); any
//! other value is a clap parse error, not a runtime finding.

use std::str::FromStr;

/// One of the four verify profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerifyMode {
    /// Quick local subset.
    Fast,
    /// Default dev profile.
    #[default]
    Local,
    /// Headless mechanical CI profile.
    Ci,
    /// OcentraParent-parity superset.
    Parent,
}

impl VerifyMode {
    /// All four values, in declaration order (used by tests/help text).
    pub const ALL: [VerifyMode; 4] = [Self::Fast, Self::Local, Self::Ci, Self::Parent];
}

impl FromStr for VerifyMode {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, String> {
        match raw.trim() {
            "" | "local" => Ok(Self::Local),
            "fast" => Ok(Self::Fast),
            "ci" => Ok(Self::Ci),
            "parent" => Ok(Self::Parent),
            other => Err(format!("Unknown verify mode: {other}")),
        }
    }
}

impl VerifyMode {
    /// Static name, used by both [`std::fmt::Display`] and
    /// `clap::ValueEnum::to_possible_value`.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Local => "local",
            Self::Ci => "ci",
            Self::Parent => "parent",
        }
    }
}

impl std::fmt::Display for VerifyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// clap derives `ValueEnum`-shaped parsing from `FromStr`/`Display` via
// `#[arg(value_parser = ...)]` normally, but since every other value in
// this crate uses the simpler pattern of implementing `clap::ValueEnum`
// directly, do the same here for a consistent parse-error message shape.
impl clap::ValueEnum for VerifyMode {
    fn value_variants<'a>() -> &'a [Self] {
        &Self::ALL
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::VerifyMode;
    use std::str::FromStr;

    #[test]
    fn empty_string_coerces_to_local() {
        assert_eq!(VerifyMode::from_str("").ok(), Some(VerifyMode::Local));
    }

    #[test]
    fn default_is_local() {
        assert_eq!(VerifyMode::default(), VerifyMode::Local);
    }

    #[test]
    fn every_known_mode_round_trips_through_display_and_from_str(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for mode in VerifyMode::ALL {
            let rendered = mode.to_string();
            let parsed = VerifyMode::from_str(&rendered)?;
            assert_eq!(parsed, mode);
        }
        Ok(())
    }

    #[test]
    fn unknown_value_is_a_hard_parse_error() -> Result<(), Box<dyn std::error::Error>> {
        match VerifyMode::from_str("bogus") {
            Err(err) => {
                assert_eq!(err, "Unknown verify mode: bogus");
                Ok(())
            }
            Ok(_) => Err("expected an error for an unknown verify mode".into()),
        }
    }
}
