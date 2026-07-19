//! Filesystem adapter for the canonical journal path brand.
//!
//! BOUNDARY-INVARIANT: filesystem paths are derived only from a validated
//! `JournalPath`; a decode conversion prevents raw paths from entering journal
//! runtime state directly.
//! BOUNDARY-TEST: invalid and malformed journal path input is rejected by the
//! journal fixture and persistence tests before this adapter is constructed.
//! boundaryOwnerNote: enforcer-events owns journal filesystem path conversion.

use std::path::{Path, PathBuf};

use enforcer_domain::events_types::JournalPath;

/// Filesystem representation retained only at the journal I/O boundary.
#[derive(Clone, Debug)]
pub(crate) struct JournalFilePath {
    domain: JournalPath,
    file: PathBuf,
}

impl JournalFilePath {
    pub(crate) fn new(domain: JournalPath) -> Self {
        let file = PathBuf::from(domain.as_str());
        Self { domain, file }
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.file.as_path()
    }

    pub(crate) fn domain(&self) -> &JournalPath {
        &self.domain
    }
}
