//! Build-time generator for the JSON-owned language registry.
//!
//! BOUNDARY-INVARIANT: this script only validates the reviewed manifest and
//! writes deterministic Rust records to OUT_DIR; it does not parse source.
//!
//! INVALID-INPUT COVERAGE: malformed manifest shape fails the build before
//! any registry source is emitted.

#[path = "language_registry.rs"]
mod language_registry;

use language_registry::{render_source, validate_source, ManifestError};
use std::env;
use std::fs;
use std::path::PathBuf;

const REVIEWED_MANIFEST: &str = include_str!("../../registry/languages.json");

fn main() -> Result<(), ManifestError> {
    validate_source(REVIEWED_MANIFEST)?;
    let generated = render_source(REVIEWED_MANIFEST)?;
    let output_directory =
        PathBuf::from(env::var_os("OUT_DIR").ok_or(ManifestError::MissingOutputDirectory)?);
    fs::write(output_directory.join("language_registry.rs"), generated)
        .map_err(|error| ManifestError::Io(error.to_string()))?;
    Ok(())
}
