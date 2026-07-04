//! Branded identifier newtypes. Each validates on construction and has no
//! public raw-string constructor; parse at the boundary, use the brand
//! everywhere after.

use enforcer_core::error::DecodeError;

/// Declare a branded string newtype with a validation function, serde
/// parse-at-boundary wiring, and accessors.
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
    let prefix = parts.next().unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::{CausationId, CorrelationId, HubName, LaneId, RuleId, ThreatId};
    use enforcer_core::error::DecodeError;

    fn parse<T: std::str::FromStr<Err = DecodeError>>(raw: &str) -> Result<T, DecodeError> {
        raw.parse()
    }

    #[test]
    fn rule_id_accepts_valid_and_rejects_malformed() -> Result<(), DecodeError> {
        for good in ["RR-6.1", "DEP-1.1", "SEC-2", "T1-a.b", "AI2-x"] {
            let id: RuleId = parse(good)?;
            assert_eq!(id.as_str(), good);
        }
        for bad in ["", "rr-6.1", "RR", "RR-", "-6.1", "RR 6.1", "RR-6 1"] {
            assert!(parse::<RuleId>(bad).is_err(), "should reject {bad:?}");
        }
        Ok(())
    }

    #[test]
    fn hub_and_lane_ids_validate() -> Result<(), DecodeError> {
        let hub: HubName = parse("enforcer-rust-build")?;
        assert_eq!(hub.as_str(), "enforcer-rust-build");
        for bad in ["", "Enforcer", "has space", "-lead", "trail-"] {
            assert!(parse::<HubName>(bad).is_err(), "should reject {bad:?}");
        }
        let lane: LaneId = parse("arc-02")?;
        assert_eq!(lane.as_str(), "arc-02");
        assert!(parse::<LaneId>("UPPER").is_err());
        Ok(())
    }

    #[test]
    fn correlation_and_causation_ids_validate() -> Result<(), DecodeError> {
        let c: CorrelationId = parse("run-2026.07.04_1234")?;
        assert_eq!(c.as_str(), "run-2026.07.04_1234");
        assert!(parse::<CorrelationId>("").is_err());
        assert!(parse::<CorrelationId>("has space").is_err());
        let long = "x".repeat(129);
        assert!(parse::<CausationId>(&long).is_err());
        Ok(())
    }

    #[test]
    fn threat_id_accepts_mitre_cwe_owasp_and_rejects_junk() -> Result<(), DecodeError> {
        for good in ["T1059", "T1059.001", "CWE-79", "A03:2021"] {
            let id: ThreatId = parse(good)?;
            assert_eq!(id.as_str(), good);
        }
        for bad in ["", "T105", "T1059.1", "CWE-", "A3:2021", "A03-2021", "X99"] {
            assert!(parse::<ThreatId>(bad).is_err(), "should reject {bad:?}");
        }
        Ok(())
    }

    #[test]
    fn serde_rejects_malformed_at_the_boundary() {
        let outcome = serde_json::from_str::<RuleId>("\"not a rule id\"");
        assert!(outcome.is_err());
    }

    #[test]
    fn serde_round_trips_valid_ids() -> Result<(), serde_json::Error> {
        let id: RuleId = serde_json::from_str("\"RR-6.1\"")?;
        let wire = serde_json::to_string(&id)?;
        assert_eq!(wire, "\"RR-6.1\"");
        Ok(())
    }
}
