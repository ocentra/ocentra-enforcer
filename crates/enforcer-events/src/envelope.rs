//! `EventEnvelope<E>` — wraps a typed [`crate::event::DomainEvent`] payload
//! with correlation/causation ids, a schema version, and a SHA-256 payload
//! digest that DECODING RE-VERIFIES: a version- or digest-drifted envelope
//! is rejected at decode time rather than silently accepted with stale or
//! tampered contents.
//!
//! See the `lib.rs` module doc for the vendoring-attribution note: this is
//! an attributed reimplementation of the envelope shape the arc-25 workpack
//! specifies, built to the same behavioral contract in the absence of the
//! canonical OcentraParent `ocentra-eventing` source.

use enforcer_core::error::DecodeError;
use enforcer_domain::ids::{CausationId, CorrelationId};

use crate::event::DomainEvent;

/// Current envelope schema version. Bumped only on a wire-incompatible
/// envelope shape change; decode rejects anything else.
pub const ENVELOPE_SCHEMA_VERSION: u32 = 1;

/// The on-the-wire envelope form: schema version, ids, event kind, the
/// serialized payload, and its digest. This is what actually gets
/// (de)serialized; [`EventEnvelope`] wraps it with a decode-time contract
/// check so callers never observe a digest-drifted instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEnvelope {
    schema_version: u32,
    correlation_id: CorrelationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    causation_id: Option<CausationId>,
    event_kind: String,
    payload: serde_json::Value,
    /// `sha256:<64 lowercase hex>` digest of the canonical JSON payload
    /// bytes, computed the same way as `enforcer_core::hash_chain`'s digest
    /// form (`sha256:` prefix + lowercase hex) but kept as a plain string
    /// here since it addresses payload content, not a hash-chain link.
    payload_digest: String,
}

/// A typed, contract-verified event envelope.
///
/// Construct with [`EventEnvelope::new`]; serialize with
/// [`EventEnvelope::to_json`]; deserialize with [`EventEnvelope::from_json`]
/// — the latter RE-VERIFIES the payload digest and schema version, so a
/// tampered or drifted envelope is rejected rather than silently accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEnvelope<E> {
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    payload: E,
}

fn digest_payload(payload_json: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload_json.as_bytes());
    let hex = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

impl<E> EventEnvelope<E>
where
    E: DomainEvent,
{
    /// Build a new envelope around a typed payload.
    pub fn new(
        correlation_id: CorrelationId,
        causation_id: Option<CausationId>,
        payload: E,
    ) -> Self {
        Self {
            correlation_id,
            causation_id,
            payload,
        }
    }

    /// The flow correlation id.
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// The optional causing-event id.
    pub fn causation_id(&self) -> Option<&CausationId> {
        self.causation_id.as_ref()
    }

    /// Borrow the typed payload.
    pub fn payload(&self) -> &E {
        &self.payload
    }

    /// Consume the envelope, returning the typed payload.
    pub fn into_payload(self) -> E {
        self.payload
    }

    /// Serialize to the wire JSON form, stamping the current schema version
    /// and a digest over the payload's canonical JSON bytes.
    pub fn to_json(&self) -> Result<String, DecodeError> {
        let payload_value = serde_json::to_value(&self.payload)
            .map_err(|e| DecodeError::new("envelope.payload", e.to_string()))?;
        let payload_json = serde_json::to_string(&payload_value)
            .map_err(|e| DecodeError::new("envelope.payload", e.to_string()))?;
        let wire = WireEnvelope {
            schema_version: ENVELOPE_SCHEMA_VERSION,
            correlation_id: self.correlation_id.clone(),
            causation_id: self.causation_id.clone(),
            event_kind: self.payload.event_kind().to_owned(),
            payload: payload_value,
            payload_digest: digest_payload(&payload_json),
        };
        serde_json::to_string(&wire).map_err(|e| DecodeError::new("envelope", e.to_string()))
    }

    /// Deserialize from the wire JSON form. RE-VERIFIES the schema version
    /// and the payload digest; a version- or digest-drifted envelope is
    /// rejected with a [`DecodeError`] rather than silently accepted.
    pub fn from_json(raw: &str) -> Result<Self, DecodeError> {
        let wire: WireEnvelope =
            serde_json::from_str(raw).map_err(|e| DecodeError::new("envelope", e.to_string()))?;

        if wire.schema_version != ENVELOPE_SCHEMA_VERSION {
            return Err(DecodeError::new(
                "envelope.schemaVersion",
                format!(
                    "expected schema version {ENVELOPE_SCHEMA_VERSION}, found {}",
                    wire.schema_version
                ),
            ));
        }

        let payload_json = serde_json::to_string(&wire.payload)
            .map_err(|e| DecodeError::new("envelope.payload", e.to_string()))?;
        let recomputed = digest_payload(&payload_json);
        if recomputed != wire.payload_digest {
            return Err(DecodeError::new(
                "envelope.payloadDigest",
                "stored payload digest does not match recomputed digest (drifted or tampered)",
            ));
        }

        let payload: E = serde_json::from_value(wire.payload)
            .map_err(|e| DecodeError::new("envelope.payload", e.to_string()))?;

        if payload.event_kind() != wire.event_kind {
            return Err(DecodeError::new(
                "envelope.eventKind",
                format!(
                    "stored eventKind `{}` does not match decoded payload kind `{}`",
                    wire.event_kind,
                    payload.event_kind()
                ),
            ));
        }

        Ok(Self {
            correlation_id: wire.correlation_id,
            causation_id: wire.causation_id,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EventEnvelope, ENVELOPE_SCHEMA_VERSION};
    use crate::event::DomainEvent;
    use enforcer_core::error::DecodeError;

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct Ping {
        n: u32,
    }

    impl DomainEvent for Ping {
        fn event_kind(&self) -> &'static str {
            "test.ping"
        }
    }

    #[test]
    fn envelope_round_trips() -> Result<(), DecodeError> {
        let envelope = EventEnvelope::new(
            "run-001".parse()?,
            Some("cause-001".parse()?),
            Ping { n: 42 },
        );
        let wire = envelope.to_json()?;
        let back: EventEnvelope<Ping> = EventEnvelope::from_json(&wire)?;
        assert_eq!(back.correlation_id().as_str(), "run-001");
        assert_eq!(back.causation_id().map(|c| c.as_str()), Some("cause-001"));
        assert_eq!(back.payload(), &Ping { n: 42 });
        Ok(())
    }

    #[test]
    fn digest_drifted_envelope_is_rejected_on_decode() -> Result<(), DecodeError> {
        let envelope = EventEnvelope::new("run-002".parse()?, None, Ping { n: 1 });
        let wire = envelope.to_json()?;
        let mut value: serde_json::Value =
            serde_json::from_str(&wire).map_err(|e| DecodeError::new("test", e.to_string()))?;
        // Tamper with the payload without updating the stored digest.
        value["payload"]["n"] = serde_json::json!(999);
        let tampered = value.to_string();
        let outcome: Result<EventEnvelope<Ping>, DecodeError> = EventEnvelope::from_json(&tampered);
        match outcome {
            Ok(_) => unreachable!("expected digest-drifted envelope to be rejected"),
            Err(err) => assert!(err.path.contains("payloadDigest")),
        }
        Ok(())
    }

    #[test]
    fn version_drifted_envelope_is_rejected_on_decode() -> Result<(), DecodeError> {
        let envelope = EventEnvelope::new("run-003".parse()?, None, Ping { n: 2 });
        let wire = envelope.to_json()?;
        let mut value: serde_json::Value =
            serde_json::from_str(&wire).map_err(|e| DecodeError::new("test", e.to_string()))?;
        value["schemaVersion"] = serde_json::json!(ENVELOPE_SCHEMA_VERSION + 1);
        let drifted = value.to_string();
        let outcome: Result<EventEnvelope<Ping>, DecodeError> = EventEnvelope::from_json(&drifted);
        match outcome {
            Ok(_) => unreachable!("expected version-drifted envelope to be rejected"),
            Err(err) => assert!(err.path.contains("schemaVersion")),
        }
        Ok(())
    }

    #[test]
    fn event_kind_mismatch_is_rejected_on_decode() -> Result<(), DecodeError> {
        let envelope = EventEnvelope::new("run-004".parse()?, None, Ping { n: 3 });
        let wire = envelope.to_json()?;
        let mut value: serde_json::Value =
            serde_json::from_str(&wire).map_err(|e| DecodeError::new("test", e.to_string()))?;
        value["eventKind"] = serde_json::json!("test.mismatch");
        // Recompute digest so the digest check itself doesn't mask this case.
        let payload_json = serde_json::to_string(&value["payload"])
            .map_err(|e| DecodeError::new("test", e.to_string()))?;
        value["payloadDigest"] = serde_json::json!(super::digest_payload(&payload_json));
        let tampered = value.to_string();
        let outcome: Result<EventEnvelope<Ping>, DecodeError> = EventEnvelope::from_json(&tampered);
        match outcome {
            Ok(_) => unreachable!("expected eventKind-mismatched envelope to be rejected"),
            Err(err) => assert!(err.path.contains("eventKind")),
        }
        Ok(())
    }

    #[test]
    fn wire_casing_is_camel_case() -> Result<(), DecodeError> {
        let envelope = EventEnvelope::new("run-005".parse()?, None, Ping { n: 4 });
        let wire = envelope.to_json()?;
        let value: serde_json::Value =
            serde_json::from_str(&wire).map_err(|e| DecodeError::new("test", e.to_string()))?;
        assert!(value.get("schemaVersion").is_some());
        assert!(value.get("correlationId").is_some());
        assert!(value.get("eventKind").is_some());
        assert!(value.get("payloadDigest").is_some());
        assert!(value.get("schema_version").is_none());
        Ok(())
    }
}
