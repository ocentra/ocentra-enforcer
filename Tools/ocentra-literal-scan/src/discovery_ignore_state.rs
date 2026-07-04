use std::path::Path;

use crate::discovery_ignore::{gitignore_pattern_matches, load_patterns};

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
