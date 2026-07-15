use enforcer_core::ndjson_writer::{read_all, NdjsonWriter};
use enforcer_domain::ids::CorrelationId;
use enforcer_domain::records::{EnforcerEvent, RunEvent, SCHEMA_VERSION};

#[test]
fn core_ndjson_sink_round_trips_domain_records() -> enforcer_core::error::Result<()> {
    let unique = format!(
        "enforcer-core-domain-records-{}-{}.ndjson",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    let path = std::env::temp_dir().join(unique);
    let event = EnforcerEvent::Run(RunEvent {
        schema_version: SCHEMA_VERSION,
        correlation_id: CorrelationId::try_from("run-001".to_owned())?,
        causation_id: None,
        epoch_ms: 1_700_000_000_000,
        tool: "cargo".to_owned(),
        exit_code: 0,
        duration_ms: 1234,
    });
    {
        let mut sink: NdjsonWriter<EnforcerEvent> = NdjsonWriter::open(&path)?;
        sink.append(&event)?;
    }
    let back: Vec<EnforcerEvent> = read_all(&path)?;
    assert_eq!(back, vec![event]);
    std::fs::remove_file(&path)?;
    Ok(())
}
