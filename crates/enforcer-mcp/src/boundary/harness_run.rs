//! Raw MCP decoding for the frozen harness `run` and `prune_runs` tools.
//!
//! BOUNDARY-INVARIANT: transport JSON is rejected here and the application
//! layer receives only canonical domain values.

use enforcer_domain::{
    config_types::CrateName,
    harness_types::{
        HarnessCommandArgument, HarnessDomainName, HarnessLanguage, HarnessPackageName,
        HarnessRunId, HarnessTag, HarnessToolName,
    },
    paths::RepoRoot,
};

/// Canonical request decoded from `ocentra_enforcer_run` MCP JSON.
#[derive(Debug, Clone)]
pub struct HarnessRunRequest {
    pub repo_root: RepoRoot,
    pub cwd: Option<String>,
    pub run_id: HarnessRunId,
    pub tool: HarnessToolName,
    pub language: Option<HarnessLanguage>,
    pub command: Vec<HarnessCommandArgument>,
    pub crate_name: Option<CrateName>,
    pub package_name: Option<HarnessPackageName>,
    pub domain: Option<HarnessDomainName>,
    pub tags: Vec<HarnessTag>,
}

/// Canonical request decoded from `ocentra_enforcer_prune_runs` MCP JSON.
#[derive(Debug, Clone)]
pub struct HarnessPruneRequest {
    pub repo_root: RepoRoot,
}

/// Decode the frozen run contract without allowing unrecognized fields.
pub(crate) fn decode_run(args: &serde_json::Value) -> Result<HarnessRunRequest, String> {
    const FIELDS: &[&str] = &[
        "root",
        "profile",
        "tool",
        "language",
        "cwd",
        "runId",
        "crateName",
        "packageName",
        "domain",
        "command",
        "tags",
    ];
    reject_unknown_fields(args, FIELDS, "run")?;
    let repo_root = decode_root(args, "run")?;
    let command = match args.get("command").and_then(serde_json::Value::as_array) {
        Some(values) if !values.is_empty() => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "run `command` entries must be strings".to_owned())
                    .and_then(|text| {
                        HarnessCommandArgument::try_new(text.to_owned())
                            .map_err(|error| format!("invalid run command: {error}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("run requires a non-empty `command` array".to_owned()),
    };
    let run_id = optional_text(args, "runId", "run")?
        .map(|text| HarnessRunId::try_new(text.to_owned()).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_else(|| {
            HarnessRunId::from_adapter(&format!(
                "run-{}",
                enforcer_core::platform::epoch_millis().unwrap_or(0)
            ))
        });
    let tool = match optional_text(args, "tool", "run")? {
        Some(text) => {
            HarnessToolName::try_new(text.to_owned()).map_err(|error| error.to_string())?
        }
        None => match command.first() {
            Some(value) => HarnessToolName::from_adapter(value.as_str()),
            None => return Err("run requires a non-empty `command` array".to_owned()),
        },
    };
    let language = match optional_text(args, "language", "run")? {
        Some("rust") => Some(HarnessLanguage::Rust),
        Some("typescript") => Some(HarnessLanguage::Typescript),
        Some("python") => Some(HarnessLanguage::Python),
        Some("common") => Some(HarnessLanguage::Common),
        Some(_) => return Err("run `language` is invalid".to_owned()),
        None => None,
    };
    let tags = match args.get("tags") {
        None => Vec::new(),
        Some(values) => values
            .as_array()
            .ok_or_else(|| "run `tags` must be an array".to_owned())?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "run `tags` entries must be strings".to_owned())
                    .and_then(|text| {
                        HarnessTag::try_new(text.to_owned())
                            .map_err(|error| format!("invalid run tag: {error}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(HarnessRunRequest {
        repo_root,
        cwd: optional_text(args, "cwd", "run")?.map(str::to_owned),
        run_id,
        tool,
        language,
        command,
        crate_name: optional_text(args, "crateName", "run")?
            .map(|text| CrateName::try_new(text.to_owned()).map_err(|error| error.to_string()))
            .transpose()?,
        package_name: optional_text(args, "packageName", "run")?
            .map(|text| {
                HarnessPackageName::try_new(text.to_owned()).map_err(|error| error.to_string())
            })
            .transpose()?,
        domain: optional_text(args, "domain", "run")?
            .map(|text| {
                HarnessDomainName::try_new(text.to_owned()).map_err(|error| error.to_string())
            })
            .transpose()?,
        tags,
    })
}

/// Decode the frozen prune contract. Its historical optional query fields are
/// accepted by the shared query decoder; pruning itself only needs the root.
pub(crate) fn decode_prune(args: &serde_json::Value) -> Result<HarnessPruneRequest, String> {
    const FIELDS: &[&str] = &[
        "root",
        "runId",
        "limit",
        "diagnosticLimit",
        "severity",
        "status",
        "file",
        "tool",
        "crateName",
        "packageName",
        "domain",
        "tag",
        "artifact",
        "limitBytes",
    ];
    reject_unknown_fields(args, FIELDS, "prune_runs")?;
    Ok(HarnessPruneRequest {
        repo_root: decode_root(args, "prune_runs")?,
    })
}

fn reject_unknown_fields(
    args: &serde_json::Value,
    fields: &[&str],
    operation: &str,
) -> Result<(), String> {
    let object = args
        .as_object()
        .ok_or_else(|| format!("{operation} arguments must be an object"))?;
    if let Some(field) = object
        .keys()
        .find(|field| !fields.contains(&field.as_str()))
    {
        return Err(format!("{operation} does not support `{field}`"));
    }
    Ok(())
}

fn decode_root(args: &serde_json::Value, operation: &str) -> Result<RepoRoot, String> {
    match args.get("root") {
        Some(serde_json::Value::String(value)) => {
            value.parse::<RepoRoot>().map_err(|error| error.to_string())
        }
        Some(_) => Err(format!("{operation} `root` must be a string")),
        None => std::env::current_dir()
            .map_err(|error| error.to_string())
            .and_then(|path| {
                path.to_string_lossy()
                    .parse::<RepoRoot>()
                    .map_err(|error| error.to_string())
            }),
    }
}

fn optional_text<'a>(
    args: &'a serde_json::Value,
    name: &str,
    operation: &str,
) -> Result<Option<&'a str>, String> {
    match args.get(name) {
        Some(serde_json::Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(format!("{operation} `{name}` must be a string")),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_run;

    #[test]
    fn run_rejects_non_string_command_entries() -> Result<(), String> {
        let error = decode_run(&serde_json::json!({"command":["cargo", 7]}))
            .err()
            .ok_or_else(|| "non-string command entry was accepted".to_owned())?;
        assert_eq!(error, "run `command` entries must be strings");
        Ok(())
    }
}
