//! `DomainEvent` — the marker contract typed event payloads implement so
//! they can travel inside an [`crate::envelope::EventEnvelope`].
//!
//! Kept intentionally minimal: a payload is a `DomainEvent` if it is
//! serde round-trippable and carries a stable, human-readable `event_kind`
//! the envelope stamps onto the wire for routing/observability. This is the
//! ENFORCER's narrow slice of upstream `ocentra-eventing`'s `DomainEvent`
//! contract (see the `lib.rs` vendoring-attribution note) — no
//! aggregate-ordering or contract-registry concerns are folded in here.

/// A typed event payload that can be carried inside an
/// [`crate::envelope::EventEnvelope`].
///
/// Implementors must be plain-data, serde round-trippable, and report a
/// stable `event_kind` string used for wire tagging and log/observability
/// routing. There is no blanket impl: each payload type opts in explicitly
/// so the wire `eventKind` stays a deliberate, reviewed surface.
pub trait DomainEvent: serde::Serialize + for<'de> serde::Deserialize<'de> {
    /// Stable, human-readable event kind stamped onto the envelope
    /// (e.g. `"scan.completed"`, `"coordination.claim.granted"`).
    fn event_kind(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::DomainEvent;

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
    fn domain_event_reports_stable_kind() {
        let ping = Ping { n: 1 };
        assert_eq!(ping.event_kind(), "test.ping");
    }
}
