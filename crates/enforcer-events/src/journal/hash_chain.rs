use enforcer_domain::events_types::{EventErrorReason, JournalHash, JournalSequence};
use sha2::{Digest, Sha256};

use super::ndjson::NdjsonJournalEntry;
use crate::boundary::journal_phase_persistence::{JournalDispatchPhaseDto, JournalHashInputDto};
use crate::boundary::stored_event_persistence::{StoredEventEnvelope, StoredEventEnvelopeDto};

const JOURNAL_HASH_PREFIX: &str = "journal-hash:";

pub(super) fn hash_entry(
    sequence: JournalSequence,
    previous_hash: Option<&JournalHash>,
    envelope: &StoredEventEnvelope,
    phase: enforcer_domain::events_types::JournalDispatchPhase,
) -> Result<JournalHash, EventErrorReason> {
    // ALLOC-JUSTIFICATION: the outbound hash DTO owns the previous hash text while the canonical JournalHash remains borrowed.
    let input = JournalHashInputDto::new(
        sequence,
        previous_hash.map(|hash| hash.as_str().to_owned()),
        JournalDispatchPhaseDto::from(phase),
        StoredEventEnvelopeDto::from(envelope),
    );
    let bytes = match serde_json::to_vec(&input) {
        Ok(bytes) => bytes,
        // ALLOC-JUSTIFICATION: hash verification owns serializer context after the borrowed serde error is dropped.
        Err(error) => return Err(EventErrorReason::from_diagnostic(error.to_string())),
    };
    let digest = Sha256::digest(&bytes);
    let hash = format!("{JOURNAL_HASH_PREFIX}{:x}", digest);
    // CLONE-JUSTIFICATION: parsing consumes its candidate while the error retains the invalid digest.
    // CLONE-JUSTIFICATION: validation consumes the candidate while diagnostics retain it on failure.
    JournalHash::try_new(hash.clone())
        .map_err(|_decode_error| EventErrorReason::from_diagnostic(hash))
}

pub(crate) fn verify_hash_chain_entry(
    entry: &NdjsonJournalEntry,
    expected_previous_hash: &Option<JournalHash>,
) -> Result<(), EventErrorReason> {
    if entry.append.current_hash.is_none() && entry.append.previous_hash.is_none() {
        return if expected_previous_hash.is_none() {
            Ok(())
        } else {
            // ALLOC-JUSTIFICATION: corruption diagnostics own formatted sequence context after the journal entry borrow is released.
            Err(EventErrorReason::from_diagnostic(format!(
                "journal hash-chain missing current hash at sequence {}",
                entry.append.sequence
            )))
        };
    }
    if &entry.append.previous_hash != expected_previous_hash {
        // ALLOC-JUSTIFICATION: corruption diagnostics own formatted sequence context after the journal entry borrow is released.
        return Err(EventErrorReason::from_diagnostic(format!(
            "journal hash-chain previous hash mismatch at sequence {}",
            entry.append.sequence
        )));
    }
    let expected_current = hash_entry(
        entry.append.sequence,
        entry.append.previous_hash.as_ref(),
        &entry.envelope,
        entry.phase,
    )?;
    match &entry.append.current_hash {
        Some(current_hash) if current_hash == &expected_current => Ok(()),
        // ALLOC-JUSTIFICATION: corruption diagnostics own expected and received hash text beyond borrowed journal state.
        Some(current_hash) => Err(EventErrorReason::from_diagnostic(format!(
            "journal hash-chain current hash mismatch at sequence {}: expected {}, received {}",
            entry.append.sequence,
            expected_current.as_str(),
            current_hash.as_str()
        ))),
        // ALLOC-JUSTIFICATION: corruption diagnostics own formatted sequence context after the journal entry borrow is released.
        None => Err(EventErrorReason::from_diagnostic(format!(
            "journal hash-chain missing current hash at sequence {}",
            entry.append.sequence
        ))),
    }
}
