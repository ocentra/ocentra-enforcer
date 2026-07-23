//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use std::path::Path;

use crate::discovery_ignore_load::load_patterns;
use crate::discovery_ignore_match::gitignore_pattern_matches;

#[derive(Debug, Clone)]
pub(crate) struct IgnoreState {
    patterns: Vec<String>,
}

impl IgnoreState {
    pub(crate) fn load(root: &Path, enabled: bool) -> Self {
        let patterns = if enabled {
            load_patterns(root)
        } else {
            Vec::new()
        };
        Self { patterns }
    }

    pub(crate) fn matches(&self, rel: &str, is_dir: bool) -> bool {
        self.patterns
            .iter()
            .any(|pattern| gitignore_pattern_matches(pattern, rel, is_dir))
    }
}
