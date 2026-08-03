//! UL00 capability inventory acceptance test.
//! Validates a closed-source manifest schema, duplicate/extension invariants,
//! supported-state hygiene, and source-derived language-row parity.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use enforcer_syntax::registry::language_registry;
use serde::Deserialize;
use serde_json::{self, Value};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum LanguageCapabilityState {
    Proved,
    Partial,
    Unsupported,
    Blocked,
    NotApplicable,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageCapabilityProvider {
    provider: String,
    version: String,
    role: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LanguageCapabilityLayer {
    state: LanguageCapabilityState,
    evidence: Vec<String>,
    providers: Vec<LanguageCapabilityProvider>,
    #[serde(rename = "notProved")]
    not_proved: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageCapabilityRow {
    language_id: String,
    aliases: Vec<String>,
    extensions: Vec<String>,
    basenames: Vec<String>,
    discovery: LanguageCapabilityLayer,
    lexical: LanguageCapabilityLayer,
    structural: LanguageCapabilityLayer,
    graph: LanguageCapabilityLayer,
    ecosystem: LanguageCapabilityLayer,
    rules: LanguageCapabilityLayer,
    #[serde(rename = "notProved")]
    not_proved: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageCapabilityManifest {
    schema_version: String,
    generated_from: String,
    rows: Vec<LanguageCapabilityRow>,
}

const MANIFEST: &str = include_str!("../capabilities/language-capabilities.json");
const INVENTORY: &str = include_str!("../../../proof/universal-language/ul00/inventory.json");
const ALLOWED_PROVIDERS: [&str; 9] = [
    "enforcer-lang-cfml",
    "enforcer-lang-common",
    "enforcer-lang-dart",
    "enforcer-lang-py",
    "enforcer-lang-rust",
    "enforcer-lang-ts",
    "enforcer-literal-scan",
    "enforcer-memory",
    "enforcer-rules",
];

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn source_language_set() -> BTreeSet<String> {
    language_registry()
        .iter()
        .map(|record| format!("{:?}", record.parser()))
        .collect()
}

fn rules_language_set() -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let workspace = workspace_root()?;
    let rules_dir = workspace.join("rules");
    let mut languages = BTreeSet::new();

    for entry in fs::read_dir(rules_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let value: Value = serde_json::from_str(&raw)?;
        let records = value.as_array().ok_or("expected rules array")?;
        for record in records {
            let validator = match record.get("validator").and_then(|v| v.as_object()) {
                Some(v) => v,
                None => continue,
            };
            let crate_name = match validator.get("crateName").and_then(|v| v.as_str()) {
                Some(name) => name,
                None => continue,
            };
            if let Some(language_id) = match crate_name {
                "enforcer-lang-rust" => Some("rust"),
                "enforcer-lang-ts" => Some("typescript"),
                "enforcer-lang-py" => Some("python"),
                "enforcer-lang-dart" => Some("dart"),
                "enforcer-lang-cfml" => Some("coldfusion"),
                _ => None,
            } {
                languages.insert(language_id.to_owned());
            }
        }
    }

    Ok(languages)
}

fn validate_manifest_for_invariants(manifest: &LanguageCapabilityManifest) -> Result<(), String> {
    let mut language_ids = HashSet::new();
    let mut extensions: HashMap<String, String> = HashMap::new();
    let mut bad_rows = Vec::new();

    for row in &manifest.rows {
        if row.language_id.trim().is_empty()
            || row.aliases.iter().any(|alias| alias.trim().is_empty())
            || row
                .extensions
                .iter()
                .any(|extension| extension.trim().is_empty())
            || row
                .basenames
                .iter()
                .any(|basename| basename.trim().is_empty())
            || row.not_proved.is_empty()
        {
            return Err(format!(
                "`{}` has an empty identity field or no row-level notProved text",
                row.language_id
            ));
        }

        if !language_ids.insert(row.language_id.clone()) {
            bad_rows.push(format!("duplicate language_id `{}`", row.language_id));
        }

        if !row.discovery.providers.is_empty() {
            ensure_provider_reachable("discovery", &row.language_id, &row.discovery.providers)?;
        }
        if !row.lexical.providers.is_empty() {
            ensure_provider_reachable("lexical", &row.language_id, &row.lexical.providers)?;
        }
        if !row.structural.providers.is_empty() {
            ensure_provider_reachable("structural", &row.language_id, &row.structural.providers)?;
        }
        if !row.graph.providers.is_empty() {
            ensure_provider_reachable("graph", &row.language_id, &row.graph.providers)?;
        }
        if !row.ecosystem.providers.is_empty() {
            ensure_provider_reachable("ecosystem", &row.language_id, &row.ecosystem.providers)?;
        }
        if !row.rules.providers.is_empty() {
            ensure_provider_reachable("rules", &row.language_id, &row.rules.providers)?;
        }

        for ext in &row.extensions {
            let lowered = ext.to_ascii_lowercase();
            if let Some(owner) = extensions.get(&lowered) {
                bad_rows.push(format!(
                    "extension `{ext}` conflict: `{}` and `{owner}` share one language extension",
                    row.language_id
                ));
            } else {
                extensions.insert(lowered, row.language_id.clone());
            }
        }

        validate_layer_state(
            &row.discovery.state,
            &row.discovery.evidence,
            &row.discovery.providers,
            &row.discovery.not_proved,
            &row.language_id,
        )?;
        validate_layer_state(
            &row.lexical.state,
            &row.lexical.evidence,
            &row.lexical.providers,
            &row.lexical.not_proved,
            &row.language_id,
        )?;
        validate_layer_state(
            &row.structural.state,
            &row.structural.evidence,
            &row.structural.providers,
            &row.structural.not_proved,
            &row.language_id,
        )?;
        validate_layer_state(
            &row.graph.state,
            &row.graph.evidence,
            &row.graph.providers,
            &row.graph.not_proved,
            &row.language_id,
        )?;
        validate_layer_state(
            &row.ecosystem.state,
            &row.ecosystem.evidence,
            &row.ecosystem.providers,
            &row.ecosystem.not_proved,
            &row.language_id,
        )?;
        validate_layer_state(
            &row.rules.state,
            &row.rules.evidence,
            &row.rules.providers,
            &row.rules.not_proved,
            &row.language_id,
        )?;
    }

    if !bad_rows.is_empty() {
        return Err(bad_rows.join("; "));
    }
    Ok(())
}

fn validate_layer_state(
    state: &LanguageCapabilityState,
    evidence: &[String],
    providers: &[LanguageCapabilityProvider],
    layer_not_proved: &[String],
    language_id: &str,
) -> Result<(), String> {
    match state {
        LanguageCapabilityState::Proved => {
            if evidence.is_empty() {
                Err(format!("`{language_id}` has proved layer with no evidence"))
            } else if layer_not_proved.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "`{language_id}` has proved layer with residual notProved text"
                ))
            }
        }
        LanguageCapabilityState::Partial => {
            if evidence.is_empty() && layer_not_proved.is_empty() {
                Err(format!(
                    "`{language_id}` has partial layer with neither evidence nor notProved text"
                ))
            } else {
                Ok(())
            }
        }
        LanguageCapabilityState::Unsupported
        | LanguageCapabilityState::Blocked
        | LanguageCapabilityState::NotApplicable => {
            if !evidence.is_empty() || !providers.is_empty() {
                Err(format!(
                    "`{language_id}` has {state:?} layer with evidence or providers"
                ))
            } else if layer_not_proved.is_empty() {
                Err(format!(
                    "`{language_id}` has {state:?} layer with no notProved text"
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn ensure_provider_reachable(
    layer: &str,
    language_id: &str,
    providers: &[LanguageCapabilityProvider],
) -> Result<(), String> {
    for provider in providers {
        if !ALLOWED_PROVIDERS.contains(&provider.provider.as_str()) {
            return Err(format!(
                "{language_id} {layer} layer uses unsupported provider `{}`",
                provider.provider
            ));
        }
        if provider.version.trim().is_empty() {
            return Err(format!(
                "{language_id} {layer} provider `{}` has no version",
                provider.provider
            ));
        }
        if provider
            .role
            .as_deref()
            .is_some_and(|role| role.trim().is_empty())
        {
            return Err(format!(
                "{language_id} {layer} provider `{}` has an empty role",
                provider.provider
            ));
        }
    }
    Ok(())
}

#[test]
fn manifest_parses_and_meets_source_language_shape() -> Result<(), Box<dyn std::error::Error>> {
    let manifest: LanguageCapabilityManifest = serde_json::from_str(MANIFEST)?;
    assert_eq!(manifest.schema_version, "1.0.0");
    assert!(!manifest.generated_from.trim().is_empty());
    let source_languages = rules_language_set()?;
    assert!(
        !manifest.rows.is_empty(),
        "manifest must expose at least one row"
    );
    assert_eq!(
        manifest.rows.len(),
        source_languages.len(),
        "manifest rows must mirror language-bearing rule-crate sources (UL00 scope)"
    );

    for language in source_languages {
        assert!(
            manifest.rows.iter().any(|row| row.language_id == language),
            "language `{language}` missing from UL00 manifest"
        );
    }
    validate_manifest_for_invariants(&manifest).map_err(std::io::Error::other)?;
    Ok(())
}

#[test]
fn inventory_preserves_all_parser_identities_and_denominators(
) -> Result<(), Box<dyn std::error::Error>> {
    let inventory: Value = serde_json::from_str(INVENTORY)?;
    assert_eq!(
        inventory["sourceSha"],
        "e19076353d8cfc945b138311de9d4738021ec05d"
    );

    let source_languages = source_language_set();
    let rows = inventory["languages"]
        .as_array()
        .ok_or("inventory languages must be an array")?;
    let inventory_identities: BTreeSet<String> = rows
        .iter()
        .map(|row| {
            row["sourceIdentity"]
                .as_str()
                .ok_or("inventory row sourceIdentity must be a string")
                .map(ToOwned::to_owned)
        })
        .collect::<Result<_, _>>()?;
    assert_eq!(rows.len(), source_languages.len());
    assert_eq!(inventory_identities, source_languages);
    assert_eq!(inventory["denominators"]["parserVariants"]["count"], 160);
    assert_eq!(
        inventory["denominators"]["structuralLanguages"]["count"],
        156
    );
    assert_eq!(
        inventory["denominators"]["explicitNoneDispatch"]["count"],
        4
    );
    assert_eq!(
        inventory["denominators"]["grammarCargoDeclarations"]["count"],
        146
    );
    assert_eq!(inventory["denominators"]["grammarBindings"]["count"], 145);
    assert_eq!(
        inventory["denominators"]["vendorGrammarTopLevelDirs"]["count"],
        51
    );
    assert!(
        !INVENTORY.contains("\"state\": \"supported\""),
        "UL00 inventory must reject a bare supported state"
    );
    Ok(())
}

#[test]
fn manifest_rejects_unsupported_state_surface_terms() {
    let payload = r#"{
        "schemaVersion":"1.0.0",
        "generatedFrom":"unit-test",
        "rows":[{"languageId":"rust","aliases":[],"extensions":[],"basenames":[],"discovery":{"state":"supported","evidence":[],"providers":[],"notProved":[]},"lexical":{"state":"blocked","evidence":[],"providers":[],"notProved":["blocked"]},"structural":{"state":"blocked","evidence":[],"providers":[],"notProved":["blocked"]},"graph":{"state":"blocked","evidence":[],"providers":[],"notProved":["blocked"]},"ecosystem":{"state":"unsupported","evidence":[],"providers":[],"notProved":["unsupported"]},"rules":{"state":"blocked","evidence":[],"providers":[],"notProved":["blocked"]},"notProved":[]}]}"#;
    let error = serde_json::from_str::<LanguageCapabilityManifest>(payload)
        .expect_err("bare supported state must be rejected");
    assert!(
        error.to_string().contains("unknown variant"),
        "unexpected closed-state rejection: {error}"
    );
}

#[test]
fn manifest_rejects_duplicate_language_id_and_extension_conflict(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut duplicate_manifest: LanguageCapabilityManifest = serde_json::from_str(MANIFEST)?;
    let duplicate = duplicate_manifest
        .rows
        .get(0)
        .cloned()
        .ok_or("expected at least one row")?;
    duplicate_manifest.rows.push(duplicate);

    let duplicate_error = validate_manifest_for_invariants(&duplicate_manifest)
        .expect_err("duplicate language identity must be rejected");
    assert!(
        duplicate_error.contains("duplicate language_id"),
        "unexpected duplicate rejection: {duplicate_error}"
    );

    let mut extension_manifest: LanguageCapabilityManifest = serde_json::from_str(MANIFEST)?;
    let rust_index = extension_manifest
        .rows
        .iter()
        .position(|row| row.language_id == "rust")
        .ok_or("expected rust row to mutate extension conflict")?;
    let extension = extension_manifest
        .rows
        .iter()
        .find(|row| row.language_id == "typescript")
        .and_then(|row| row.extensions.first())
        .cloned()
        .ok_or("expected typescript extension")?;
    extension_manifest.rows[rust_index]
        .extensions
        .push(extension);

    let extension_error = validate_manifest_for_invariants(&extension_manifest)
        .expect_err("extension collision must be rejected");
    assert!(
        extension_error.contains("extension `ts` conflict"),
        "unexpected extension rejection: {extension_error}"
    );
    Ok(())
}

#[test]
fn manifest_rejects_missing_evidence_for_proved_layers() -> Result<(), Box<dyn std::error::Error>> {
    let mut manifest: LanguageCapabilityManifest = serde_json::from_str(MANIFEST)?;
    manifest
        .rows
        .first_mut()
        .ok_or("expected at least one row")?
        .discovery
        .evidence
        .clear();
    let error =
        validate_manifest_for_invariants(&manifest).expect_err("missing evidence must be rejected");
    assert!(
        error.contains("no evidence"),
        "unexpected missing-evidence rejection: {error}"
    );
    Ok(())
}

fn mutate_provider(row: &mut LanguageCapabilityRow, provider_id: &str) {
    if let Some(provider) = row.discovery.providers.first_mut() {
        provider.provider = provider_id.to_owned();
        return;
    }
    if let Some(provider) = row.lexical.providers.first_mut() {
        provider.provider = provider_id.to_owned();
        return;
    }
    if let Some(provider) = row.structural.providers.first_mut() {
        provider.provider = provider_id.to_owned();
        return;
    }
    if let Some(provider) = row.graph.providers.first_mut() {
        provider.provider = provider_id.to_owned();
        return;
    }
    if let Some(provider) = row.ecosystem.providers.first_mut() {
        provider.provider = provider_id.to_owned();
        return;
    }
    if let Some(provider) = row.rules.providers.first_mut() {
        provider.provider = provider_id.to_owned();
    }
}

#[test]
fn manifest_rejects_unreachable_provider_ids() -> Result<(), Box<dyn std::error::Error>> {
    let mut manifest: LanguageCapabilityManifest = serde_json::from_str(MANIFEST)?;
    let row = manifest
        .rows
        .get_mut(0)
        .ok_or("expected at least one row")?;
    mutate_provider(row, "non-existent-provider");
    let error = validate_manifest_for_invariants(&manifest)
        .expect_err("unreachable provider must be rejected");
    assert!(
        error.contains("unsupported provider"),
        "unexpected provider rejection: {error}"
    );
    Ok(())
}
