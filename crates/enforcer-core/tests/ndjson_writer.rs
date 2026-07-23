use enforcer_core::error::Result;
use enforcer_core::ndjson_writer::{read_all, NdjsonWriter};

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Record {
    seq: u32,
    event: String,
}

fn temp_path(name: &str) -> std::path::PathBuf {
    let unique = format!(
        "enforcer-core-ndjson-{}-{}-{name}.ndjson",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    );
    std::env::temp_dir().join(unique)
}

#[test]
fn append_round_trips_records() -> Result<()> {
    let path = temp_path("round-trip");
    {
        let mut writer: NdjsonWriter<Record> = NdjsonWriter::open(&path)?;
        writer.append(&Record {
            seq: 1,
            event: "start".to_owned(),
        })?;
        writer.append(&Record {
            seq: 2,
            event: "finish".to_owned(),
        })?;
    }
    let records: Vec<Record> = read_all(&path)?;
    assert_eq!(
        records,
        vec![
            Record {
                seq: 1,
                event: "start".to_owned(),
            },
            Record {
                seq: 2,
                event: "finish".to_owned(),
            },
        ]
    );
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn reopen_appends_instead_of_truncating() -> Result<()> {
    let path = temp_path("append-only");
    {
        let mut writer: NdjsonWriter<Record> = NdjsonWriter::open(&path)?;
        writer.append(&Record {
            seq: 1,
            event: "first-open".to_owned(),
        })?;
    }
    {
        let mut writer: NdjsonWriter<Record> = NdjsonWriter::open(&path)?;
        writer.append(&Record {
            seq: 2,
            event: "second-open".to_owned(),
        })?;
    }
    let records: Vec<Record> = read_all(&path)?;
    assert_eq!(records.len(), 2, "reopening must never truncate");
    assert_eq!(records[0].event, "first-open");
    assert_eq!(records[1].event, "second-open");
    std::fs::remove_file(&path)?;
    Ok(())
}

#[test]
fn malformed_record_is_rejected_at_the_decode_boundary(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let path = temp_path("malformed");
    std::fs::write(&path, "{\"seq\":1\n")?;
    let error = match read_all::<Record>(&path) {
        Ok(_) => return Err("malformed NDJSON was unexpectedly accepted".into()),
        Err(error) => error,
    };
    assert!(matches!(error, enforcer_core::error::Error::Json(_)));
    std::fs::remove_file(&path)?;
    Ok(())
}
