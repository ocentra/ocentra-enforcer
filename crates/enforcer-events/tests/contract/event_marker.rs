use enforcer_events::event::DomainEvent;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Ping {
    // BRAND-INVARIANT: test payload count is an inert fixture value and never crosses an event-domain boundary.
    n: u32,
}

impl DomainEvent for Ping {
    fn event_kind(
        &self,
    ) -> Result<
        enforcer_domain::events_types::EventType,
        enforcer_domain::boundary::decode_error::DecodeError,
    > {
        enforcer_domain::events_types::EventType::parse("test.ping")
    }
}

#[test]
fn domain_event_reports_stable_kind(
) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
    let ping = Ping { n: 1 };
    assert_eq!(ping.event_kind()?.as_str(), "test.ping");
    Ok(())
}
