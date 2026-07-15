//! Branded identifier newtypes. Each validates on construction and has no
//! public raw-string constructor; parse at the boundary, use the brand
//! everywhere after.

use crate::boundary::decode_error::DecodeError;

/// Declare a branded string newtype with validation and serde boundary wiring.
macro_rules! branded_string {
    ($(#[$doc:meta])* $name:ident, $field_path:literal, $validate:path) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
            serde::Serialize, serde::Deserialize, ts_rs::TS,
        )]
        #[serde(try_from = "String", into = "String")]
        #[ts(type = "string")]
        pub struct $name(String);

        impl $name {
            /// View the validated inner value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(raw: String) -> Result<Self, DecodeError> {
                $validate(&raw)?;
                Ok(Self(raw))
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;

            fn from_str(raw: &str) -> Result<Self, DecodeError> {
                // ALLOC-JUSTIFICATION: each brand owns the validated value so it
                // remains valid across event and async transport boundaries.
                Self::try_from(raw.to_owned())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

fn validate_rule_id(raw: &str) -> Result<(), DecodeError> {
    // e.g. `RR-6.1`, `DEP-1.1`, `SEC-2.3`: uppercase alnum family prefix,
    // then dash-separated alnum/dot segments.
    let mut parts = raw.split('-');
    let Some(prefix) = parts.next() else {
        return Err(DecodeError::new(
            "ruleId",
            "expected `PREFIX-segment[...]` with uppercase alnum prefix (e.g. `RR-6.1`)",
        ));
    };
    let prefix_ok = !prefix.is_empty()
        && prefix
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        && prefix
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let mut rest_count = 0usize;
    let mut rest_ok = true;
    for segment in parts {
        rest_count += 1;
        if segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.')
        {
            rest_ok = false;
        }
    }
    if prefix_ok && rest_count > 0 && rest_ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "ruleId",
            "expected `PREFIX-segment[...]` with uppercase alnum prefix (e.g. `RR-6.1`)",
        ))
    }
}

fn validate_hub_name(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 128
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !raw.starts_with('-')
        && !raw.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "hubName",
            "expected lowercase kebab-case (e.g. `enforcer-rust-build`)",
        ))
    }
}

fn validate_lane_id(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && !raw.starts_with('-')
        && !raw.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "laneId",
            "expected lowercase alnum/dash/underscore (e.g. `arc-02`)",
        ))
    }
}

fn validate_harness_id(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "harnessId",
            "expected lowercase kebab-case (e.g. `claude`, `codex`, `kilocode`)",
        ))
    }
}

fn validate_correlation_like(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 128
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "correlationId",
            "expected 1..=128 chars of alnum/dash/underscore/dot",
        ))
    }
}

fn validate_threat_id(raw: &str) -> Result<(), DecodeError> {
    // MITRE ATT&CK technique (`T1059` / `T1059.001`), CWE (`CWE-79`), or
    // OWASP Top-10 slot (`A03:2021`).
    let mitre = raw.strip_prefix('T').is_some_and(|rest| {
        let mut halves = rest.splitn(2, '.');
        let base = halves.next().unwrap_or_default();
        let sub = halves.next();
        base.len() == 4
            && base.chars().all(|c| c.is_ascii_digit())
            && sub.is_none_or(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()))
    });
    let cwe = raw
        .strip_prefix("CWE-")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()));
    let owasp = raw.strip_prefix('A').is_some_and(|rest| {
        let mut halves = rest.splitn(2, ':');
        let slot = halves.next().unwrap_or_default();
        let year = halves.next();
        slot.len() == 2
            && slot.chars().all(|c| c.is_ascii_digit())
            && year.is_some_and(|y| y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()))
    });
    if mitre || cwe || owasp {
        Ok(())
    } else {
        Err(DecodeError::new(
            "threatId",
            "expected MITRE `T####[.###]`, `CWE-#`, or OWASP `A##:####`",
        ))
    }
}

fn validate_github_check_context(raw: &str) -> Result<(), DecodeError> {
    let valid = !raw.is_empty()
        && raw.len() <= 512
        && raw
            .chars()
            .all(|character| !character.is_control() && character != '\n' && character != '\r');
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new(
            "githubCheckContext",
            "expected 1..=512 printable characters without line breaks",
        ))
    }
}

fn validate_github_branch_name(raw: &str) -> Result<(), DecodeError> {
    let valid = !raw.is_empty()
        && raw.len() <= 255
        && !raw.starts_with('-')
        && !raw.ends_with('/')
        && !raw.contains("..")
        && raw.chars().all(|character| {
            !character.is_control()
                && !character.is_whitespace()
                && character != '~'
                && character != '^'
                && character != ':'
                && character != '?'
                && character != '*'
                && character != '['
                && character != '\\'
        });
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new(
            "githubBranchName",
            "expected a non-empty GitHub branch name without control, whitespace, or Git ref special characters",
        ))
    }
}

branded_string!(
    /// Branded rule identifier (e.g. `RR-6.1`, `DEP-1.1`).
    RuleId,
    "ruleId",
    validate_rule_id
);

branded_string!(
    /// Branded coordination hub name (e.g. `enforcer-rust-build`).
    HubName,
    "hubName",
    validate_hub_name
);

branded_string!(
    /// Branded coordination lane id (e.g. `arc-02`).
    LaneId,
    "laneId",
    validate_lane_id
);

branded_string!(
    /// Branded agent-harness identifier (e.g. `claude`, `codex`, `kilocode`).
    HarnessId,
    "harnessId",
    validate_harness_id
);

branded_string!(
    /// Branded correlation id stitching one logical flow across crates.
    CorrelationId,
    "correlationId",
    validate_correlation_like
);

branded_string!(
    /// Branded causation id linking an event to the event that caused it.
    CausationId,
    "causationId",
    validate_correlation_like
);

branded_string!(
    /// Branded threat identifier: MITRE ATT&CK, CWE, or OWASP Top-10.
    ThreatId,
    "threatId",
    validate_threat_id
);

branded_string!(
    /// Branded GitHub status-check context, validated before protection policy comparison.
    GitHubCheckContext,
    "githubCheckContext",
    validate_github_check_context
);

branded_string!(
    /// Branded GitHub branch name used by branch-protection policy and reports.
    GitHubBranchName,
    "githubBranchName",
    validate_github_branch_name
);

#[cfg(test)]
mod tests {
    use super::{GitHubBranchName, GitHubCheckContext};

    #[test]
    fn github_check_context_rejects_line_breaks() {
        assert!(GitHubCheckContext::try_from("Rust CI / rust-ci\nspoof".to_owned()).is_err());
    }

    #[test]
    fn github_branch_name_rejects_git_ref_special_characters() {
        assert!(GitHubBranchName::try_from("main..backup".to_owned()).is_err());
        assert!(GitHubBranchName::try_from("release candidate".to_owned()).is_err());
    }

    #[test]
    fn github_branch_protection_values_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let context =
            GitHubCheckContext::try_from("Rust CI / rust-ci (windows-latest)".to_owned())?;
        let branch = GitHubBranchName::try_from("main".to_owned())?;
        assert_eq!(context.as_str(), "Rust CI / rust-ci (windows-latest)");
        assert_eq!(branch.as_str(), "main");
        Ok(())
    }
}
