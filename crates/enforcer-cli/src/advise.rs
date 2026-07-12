//! The `advise` CLI seam onto arc-13's literal-risk engine
//! (`enforcer-literal-scan`). Today accepts exactly one target,
//! `literals`; any other target is a hard usage error (parity with the
//! legacy Node `handleAdviseCommand`'s "advise currently supports only
//! literals").

use std::str::FromStr;

/// The advise target. `Literals` is the only value that exists today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdviseTarget {
    /// Route to the arc-13 literal-risk check.
    Literals,
}

/// A rejected `advise` target at the CLI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdviseTargetError {
    /// The input did not name the only supported target, `literals`.
    UnsupportedTarget { raw: String },
}

impl std::fmt::Display for AdviseTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTarget { raw } => {
                write!(f, "advise currently supports only literals (got `{raw}`)")
            }
        }
    }
}

impl std::error::Error for AdviseTargetError {}

impl FromStr for AdviseTarget {
    type Err = AdviseTargetError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "literals" => Ok(Self::Literals),
            other => Err(AdviseTargetError::UnsupportedTarget {
                raw: other.to_owned(),
            }),
        }
    }
}

impl AdviseTarget {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Literals => "literals",
        }
    }
}

impl std::fmt::Display for AdviseTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl clap::ValueEnum for AdviseTarget {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Literals]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::AdviseTarget;
    use std::str::FromStr;

    #[test]
    fn literals_parses() {
        assert_eq!(
            AdviseTarget::from_str("literals"),
            Ok(AdviseTarget::Literals)
        );
    }

    #[test]
    fn any_other_target_is_a_hard_error() {
        assert!(AdviseTarget::from_str("secrets").is_err());
        assert!(AdviseTarget::from_str("").is_err());
    }
}
