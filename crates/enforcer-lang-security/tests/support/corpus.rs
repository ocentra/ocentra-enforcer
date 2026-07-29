use std::path::Path;

#[derive(serde::Deserialize)]
pub(crate) struct Case {
    pub(crate) name: String,
    pub(crate) input: String,
    pub(crate) expect: String,
    // DEFAULT-JUSTIFICATION: branch metadata is optional test-case documentation.
    #[serde(default)]
    pub(crate) branch: String,
    // DEFAULT-JUSTIFICATION: reason metadata is optional test-case documentation.
    #[serde(default)]
    pub(crate) reason: String,
}

pub(crate) fn load(
    manifest_dir: &Path,
    corpus_file: &str,
) -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    let path = manifest_dir
        .join("tests/fixtures/cyberskills/_corpus")
        .join(corpus_file);
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read corpus {}: {error}", path.display()))?;
    Ok(serde_json::from_str(&raw)?)
}
