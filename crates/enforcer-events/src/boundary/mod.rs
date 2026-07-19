//! Transport and persistence edges for `enforcer-events`.
//!
//! BOUNDARY-INVARIANT: raw wire, filesystem, and presentation inputs are
//! passed through a decode conversion and validated into event-domain values
//! before they enter runtime
//! policy or dispatch logic.
//! BOUNDARY-TEST: invalid and malformed boundary inputs are rejected by the
//! contract, journal, topology, and request persistence suites.
//! boundaryOwnerNote: enforcer-events owns these transport conversion seams.

pub(crate) mod envelope_persistence;
pub mod event_contract_persistence;
pub mod event_metadata_persistence;
pub(crate) mod event_values;
pub(crate) mod journal_file_path;
pub mod journal_persistence;
pub mod journal_phase_persistence;
pub mod request_persistence;
pub mod stored_event_persistence;
pub mod topology_contract_presentation;
pub mod topology_presentation;
