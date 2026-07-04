use std::fs;
use std::path::Path;

pub(crate) fn load_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    for rel in [".gitignore", ".ignore", ".git/info/exclude"] {
        patterns.extend(read_patterns_from_file(&root.join(rel)));
    }
    patterns
}

fn read_patterns_from_file(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines().filter_map(normalize_pattern_line).collect()
}

fn normalize_pattern_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return None;
    }
    Some(trimmed.trim_start_matches('/').to_string())
}
