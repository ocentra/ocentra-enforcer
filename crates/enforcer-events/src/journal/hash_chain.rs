use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{EventingError, JournalDispatchPhase, JournalHash, StoredEventEnvelope};

use super::ndjson::NdjsonJournalEntry;

const JOURNAL_HASH_PREFIX: &str = "journal-hash:";

#[derive(Serialize)]
struct JournalHashInput<'a> {
    sequence: u64,
    previous_hash: Option<&'a JournalHash>,
    phase: JournalDispatchPhase,
    envelope: &'a StoredEventEnvelope,
}

pub(super) fn hash_entry(
    sequence: u64,
    previous_hash: Option<&JournalHash>,
    envelope: &StoredEventEnvelope,
    phase: JournalDispatchPhase,
) -> Result<JournalHash, EventingError> {
    let input = JournalHashInput {
        sequence,
        previous_hash,
        phase,
        envelope,
    };
    let bytes =
        serde_json::to_vec(&input).map_err(|error| EventingError::journal_encode(&error))?;
    let digest = Sha256::digest(&bytes);
    JournalHash::parse(format!("{JOURNAL_HASH_PREFIX}{:x}", digest))
}

pub(crate) fn verify_hash_chain_entry(
    entry: &NdjsonJournalEntry,
    expected_previous_hash: &Option<JournalHash>,
) -> Result<(), String> {
    if entry.append.current_hash.is_none() && entry.append.previous_hash.is_none() {
        return if expected_previous_hash.is_none() {
            Ok(())
        } else {
            Err(format!(
                "journal hash-chain missing current hash at sequence {}",
                entry.append.sequence
            ))
        };
    }
    if &entry.append.previous_hash != expected_previous_hash {
        return Err(format!(
            "journal hash-chain previous hash mismatch at sequence {}",
            entry.append.sequence
        ));
    }
    let expected_current = hash_entry(
        entry.append.sequence,
        entry.append.previous_hash.as_ref(),
        &entry.envelope,
        entry.phase,
    )
    .map_err(|error| error.to_string())?;
    match &entry.append.current_hash {
        Some(current_hash) if current_hash == &expected_current => Ok(()),
        Some(current_hash) => Err(format!(
            "journal hash-chain current hash mismatch at sequence {}: expected {}, received {}",
            entry.append.sequence,
            expected_current.as_str(),
            current_hash.as_str()
        )),
        None => Err(format!(
            "journal hash-chain missing current hash at sequence {}",
            entry.append.sequence
        )),
    }
}
