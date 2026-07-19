//! Black-box tests for branded identifier validation and serde boundaries.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::events_types::{
    EventCount, EventDeliveryCapabilityState, EventDeliveryClaimState, JournalSequence,
};
use enforcer_domain::ids::{
    CausationId, CorrelationId, HarnessId, HubName, LaneId, RuleId, ThreatId,
};

fn parse<T: std::str::FromStr<Err = DecodeError>>(raw: &str) -> Result<T, DecodeError> {
    raw.parse()
}

fn assert_rejected<T: std::str::FromStr<Err = DecodeError>>(
    raw: &str,
    path: &str,
) -> Result<(), DecodeError> {
    match parse::<T>(raw) {
        Err(error) => {
            assert_eq!(error.path, path);
            assert_ne!(error.reason, "");
            Ok(())
        }
        Ok(_) => Err(DecodeError::new(
            path,
            "expected invalid input to be rejected",
        )),
    }
}

fn serde_rejection<T: serde::de::DeserializeOwned>(
    raw: &str,
    path: &str,
) -> Result<serde_json::Error, DecodeError> {
    match serde_json::from_str::<T>(raw) {
        Err(error) => Ok(error),
        Ok(_) => Err(DecodeError::new(path, "expected JSON value to be rejected")),
    }
}

#[test]
fn rule_id_accepts_valid_and_rejects_malformed() -> Result<(), DecodeError> {
    for good in ["RR-6.1", "DEP-1.1", "SEC-2", "T1-a.b", "AI2-x"] {
        let id: RuleId = parse(good)?;
        assert_eq!(id.as_str(), good);
    }
    for bad in ["", "rr-6.1", "RR", "RR-", "-6.1", "RR 6.1", "RR-6 1"] {
        assert_rejected::<RuleId>(bad, "ruleId")?;
    }
    Ok(())
}

#[test]
fn rule_id_required_at_a_registry_shaped_boundary_not_bare_string() -> Result<(), DecodeError> {
    use std::collections::BTreeMap;

    fn lookup<'a>(map: &'a BTreeMap<RuleId, &'static str>, id: &RuleId) -> Option<&'a str> {
        map.get(id).copied()
    }

    let mut registry: BTreeMap<RuleId, &'static str> = BTreeMap::new();
    let id: RuleId = parse("RR-6.1")?;
    registry.insert(id.clone(), "sample rule");
    assert_eq!(lookup(&registry, &id), Some("sample rule"));
    Ok(())
}

#[test]
fn hub_and_lane_ids_validate() -> Result<(), DecodeError> {
    let hub: HubName = parse("enforcer-rust-build")?;
    assert_eq!(hub.as_str(), "enforcer-rust-build");
    for bad in ["", "Enforcer", "has space", "-lead", "trail-"] {
        assert_rejected::<HubName>(bad, "hubName")?;
    }
    let lane: LaneId = parse("arc-02")?;
    assert_eq!(lane.as_str(), "arc-02");
    assert_rejected::<LaneId>("UPPER", "laneId")?;
    Ok(())
}

#[test]
fn harness_id_validates_at_the_shared_boundary() -> Result<(), DecodeError> {
    for valid in ["claude", "codex", "kilocode", "agent-2"] {
        assert_eq!(parse::<HarnessId>(valid)?.as_str(), valid);
    }
    for invalid in ["", "Codex", "has space", "a_b", "a/b"] {
        assert_rejected::<HarnessId>(invalid, "harnessId")?;
    }
    let malformed = serde_rejection::<HarnessId>("\"Not Valid\"", "harnessId")?;
    assert_eq!(malformed.classify(), serde_json::error::Category::Data);
    Ok(())
}

#[test]
fn coordination_id_brand_decode() -> Result<(), DecodeError> {
    let hub: HubName = parse("enforcer-rust-build")?;
    assert_eq!(hub.as_str(), "enforcer-rust-build");
    let lane: LaneId = parse("arc-06")?;
    assert_eq!(lane.as_str(), "arc-06");
    assert_rejected::<HubName>("", "hubName")?;
    assert_rejected::<LaneId>("", "laneId")?;
    for bad in ["../escape", "a/b", "a\\b", "UPPER", "has space", "a.b"] {
        assert_rejected::<HubName>(bad, "hubName")?;
        assert_rejected::<LaneId>(bad, "laneId")?;
    }
    let oversize_hub = "a".repeat(129);
    assert_rejected::<HubName>(&oversize_hub, "hubName")?;
    let max_hub = "a".repeat(128);
    assert_eq!(parse::<HubName>(&max_hub)?.as_str(), max_hub);
    let oversize_lane = "a".repeat(65);
    assert_rejected::<LaneId>(&oversize_lane, "laneId")?;
    let max_lane = "a".repeat(64);
    assert_eq!(parse::<LaneId>(&max_lane)?.as_str(), max_lane);
    let empty_hub = serde_rejection::<HubName>("\"\"", "hubName")?;
    assert_eq!(empty_hub.classify(), serde_json::error::Category::Data);
    let escaped_lane = serde_rejection::<LaneId>("\"../escape\"", "laneId")?;
    assert_eq!(escaped_lane.classify(), serde_json::error::Category::Data);
    Ok(())
}

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
    Ok(())
}

#[test]
fn correlation_and_causation_ids_validate() -> Result<(), DecodeError> {
    let c: CorrelationId = parse("run-2026.07.04_1234")?;
    assert_eq!(c.as_str(), "run-2026.07.04_1234");
    assert_rejected::<CorrelationId>("", "correlationId")?;
    assert_rejected::<CorrelationId>("has space", "correlationId")?;
    let long = "x".repeat(129);
    assert_rejected::<CausationId>(&long, "correlationId")?;
    Ok(())
}

#[test]
fn threat_id_accepts_mitre_cwe_owasp_and_rejects_junk() -> Result<(), DecodeError> {
    for good in ["T1059", "T1059.001", "CWE-79", "A03:2021"] {
        let id: ThreatId = parse(good)?;
        assert_eq!(id.as_str(), good);
    }
    for bad in ["", "T105", "T1059.1", "CWE-", "A3:2021", "A03-2021", "X99"] {
        assert_rejected::<ThreatId>(bad, "threatId")?;
    }
    Ok(())
}

#[test]
fn serde_rejects_malformed_at_the_boundary() -> Result<(), DecodeError> {
    let outcome = serde_rejection::<RuleId>("\"not a rule id\"", "ruleId")?;
    assert_eq!(outcome.classify(), serde_json::error::Category::Data);
    Ok(())
}

#[test]
fn serde_round_trips_valid_ids() -> Result<(), serde_json::Error> {
    let id: RuleId = serde_json::from_str("\"RR-6.1\"")?;
    let wire = serde_json::to_string(&id)?;
    assert_eq!(wire, "\"RR-6.1\"");
    Ok(())
}

#[test]
fn journal_sequence_requires_a_positive_position() -> Result<(), DecodeError> {
    let sequence = JournalSequence::new(42)?;
    assert_eq!(sequence.as_u64(), 42);

    let invalid = match JournalSequence::new(0) {
        Err(error) => error,
        Ok(_) => {
            return Err(DecodeError::new(
                "journal_sequence",
                "zero must be rejected",
            ));
        }
    };
    assert_eq!(invalid.path, "journal_sequence");
    assert_ne!(invalid.reason, "");
    Ok(())
}

#[test]
fn event_count_tracks_monotonic_increment_and_non_negative_decrement() {
    let one = EventCount::ZERO.incremented();
    assert_eq!(one.as_nonzero(), Some(std::num::NonZeroUsize::MIN));
    assert_eq!(one.decremented().as_nonzero(), None);
    assert_eq!(EventCount::ZERO.decremented().as_nonzero(), None);
}

#[test]
fn event_delivery_states_express_claims_and_capabilities_without_raw_booleans() {
    assert_ne!(
        EventDeliveryClaimState::Claimed,
        EventDeliveryClaimState::NotClaimed
    );
    assert_ne!(
        EventDeliveryCapabilityState::Available,
        EventDeliveryCapabilityState::Unavailable
    );
}
