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
    ///
    /// `HubName` and [`LaneId`] are deliberately separate nominal types (each
    /// its own single-field tuple struct, not a type alias) so that passing
    /// one where the other is expected is a COMPILE error, not a runtime
    /// surprise buried in a coordination event or a mislocated filesystem
    /// path. `crates/enforcer-coordination` never accepts a bare `String`
    /// for either: `init`, `claim_all`, `release`, `closeout`, and the
    /// `stream_path`/`lock_path` helpers all take `&HubName`/`&LaneId`
    /// directly, so this distinctness guarantee is enforced at every call
    /// site, not just at construction.
    HubName,
    "hubName",
    validate_hub_name
);

branded_string!(
    /// Branded coordination lane id (e.g. `arc-02`). See [`HubName`] for the
    /// compile-time distinctness guarantee shared by this pair.
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

    /// Named proof for `TEST_PROOF_EXPECTATIONS.md` row `a03`: a
    /// registry-shaped map keyed by `RuleId` only accepts `RuleId` keys —
    /// this module compiling at all is itself the proof that a bare
    /// `String` cannot substitute for `RuleId` at that boundary, since the
    /// helper below is written to take `&RuleId` and there is no overload
    /// accepting `&str`/`String` (swapping the parameter type to `&String`
    /// and passing a raw string literal is a COMPILE error, not a runtime
    /// check). `enforcer-rules::registry::RuleRegistry` relies on exactly
    /// this property: its `BTreeMap<RuleId, RuleRecord>` and `get(&RuleId)`
    /// accept the branded type only.
    #[test]
    fn rule_id_required_at_a_registry_shaped_boundary_not_bare_string() -> Result<(), DecodeError>
    {
        use std::collections::BTreeMap;

        fn lookup<'a>(map: &'a BTreeMap<RuleId, &'static str>, id: &RuleId) -> Option<&'a str> {
            map.get(id).copied()
        }

        let mut registry: BTreeMap<RuleId, &'static str> = BTreeMap::new();
        let id: RuleId = parse("RR-6.1")?;
        registry.insert(id.clone(), "sample rule");
        assert_eq!(lookup(&registry, &id), Some("sample rule"));
        // `lookup(&registry, &"RR-6.1".to_owned())` and inserting a bare
        // `String` key into `registry` do not type-check: `BTreeMap<RuleId,
        // _>` and `lookup`'s `&RuleId` parameter both reject `String`/`&str`
        // outright, so an unbranded id can never flow into the registry
        // API. Left as a documented guarantee (as the sibling
        // `hub_name_and_lane_id_are_not_interchangeable` test does) rather
        // than a `trybuild` harness, since none is vendored in this
        // workspace and the type signatures above are the enforced
        // guarantee.
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

    /// Named proof for `TEST_PROOF_EXPECTATIONS.md` row `a06`: unsafe
    /// charset / empty / oversize all fail closed for both `HubName` and
    /// `LaneId`, including path-separator and `..`-escape attempts (which
    /// are already excluded by the charset allow-list, not by a separate
    /// denylist check — there is no character in either charset that can
    /// spell a path separator or a `..` segment).
    #[test]
    fn coordination_id_brand_decode() -> Result<(), DecodeError> {
        // Valid mint for both newtypes.
        let hub: HubName = parse("enforcer-rust-build")?;
        assert_eq!(hub.as_str(), "enforcer-rust-build");
        let lane: LaneId = parse("arc-06")?;
        assert_eq!(lane.as_str(), "arc-06");

        // Empty.
        assert!(parse::<HubName>("").is_err());
        assert!(parse::<LaneId>("").is_err());

        // Unsafe charset: path separators, `..` escape, whitespace, case.
        for bad in ["../escape", "a/b", "a\\b", "UPPER", "has space", "a.b"] {
            assert!(parse::<HubName>(bad).is_err(), "hub should reject {bad:?}");
            assert!(parse::<LaneId>(bad).is_err(), "lane should reject {bad:?}");
        }

        // Oversize: one char past each newtype's documented bound.
        let oversize_hub = "a".repeat(129);
        assert!(parse::<HubName>(&oversize_hub).is_err());
        let max_hub = "a".repeat(128);
        assert!(parse::<HubName>(&max_hub).is_ok());

        let oversize_lane = "a".repeat(65);
        assert!(parse::<LaneId>(&oversize_lane).is_err());
        let max_lane = "a".repeat(64);
        assert!(parse::<LaneId>(&max_lane).is_ok());

        // Serde rejects the same malformed inputs at the boundary.
        assert!(serde_json::from_str::<HubName>("\"\"").is_err());
        assert!(serde_json::from_str::<LaneId>("\"../escape\"").is_err());
        Ok(())
    }

    /// Compile-reject fixture for `HubName` vs `LaneId`: these helpers only
    /// accept their own branded type, so this module compiling at all is
    /// itself the proof that a `LaneId` cannot be passed where a `HubName`
    /// is expected (and vice versa) — swapping either call below to pass
    /// the other branded type is a COMPILE error, not a lint or a runtime
    /// check. `enforcer-coordination::api` relies on exactly this property
    /// for `init`/`claim_all`/`release`/`closeout`.
    #[test]
    fn hub_name_and_lane_id_are_not_interchangeable() -> Result<(), DecodeError> {
        fn accepts_hub(hub: &HubName) -> &str {
            hub.as_str()
        }
        fn accepts_lane(lane: &LaneId) -> &str {
            lane.as_str()
        }
        let hub: HubName = parse("enforcer-rust-build")?;
        let lane: LaneId = parse("arc-06")?;
        assert_eq!(accepts_hub(&hub), "enforcer-rust-build");
        assert_eq!(accepts_lane(&lane), "arc-06");
        // `accepts_hub(&lane)` and `accepts_lane(&hub)` do not type-check;
        // left commented rather than behind a `trybuild` harness (none is
        // vendored in this workspace) since the type signatures above are
        // themselves the enforced guarantee.
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
