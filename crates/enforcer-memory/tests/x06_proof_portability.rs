type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .components()
        .collect()
}

fn looks_like_machine_absolute_path(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    chars.windows(4).enumerate().any(|(index, window)| {
        let drive = window[0];
        if !drive.is_ascii_alphabetic() || chars[index + 1] != ':' {
            return false;
        }

        let separator = window[2];
        if separator != '\\' && separator != '/' {
            return false;
        }

        let previous = index
            .checked_sub(1)
            .map(|previous_index| chars[previous_index]);
        if previous.is_some_and(|character| character.is_ascii_alphanumeric()) {
            return false;
        }

        let next = window[3];
        next.is_ascii_alphanumeric() || matches!(next, '.' | '_' | '-' | '\\' | '/')
    })
}

fn collect_json_path_leaks(
    artifact: &str,
    json_path: &str,
    value: &serde_json::Value,
    leaks: &mut Vec<String>,
) {
    match value {
        serde_json::Value::String(value) if looks_like_machine_absolute_path(value) => {
            leaks.push(format!("{artifact}:{json_path}"));
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_json_path_leaks(artifact, &format!("{json_path}[{index}]"), item, leaks);
            }
        }
        serde_json::Value::Object(fields) => {
            for (field, item) in fields {
                let child_path = if json_path.is_empty() {
                    field.to_owned()
                } else {
                    format!("{json_path}.{field}")
                };
                collect_json_path_leaks(artifact, &child_path, item, leaks);
            }
        }
        _ => {}
    }
}

fn assert_json_portable(artifact: &str, body: &str) -> TestResult {
    let value: serde_json::Value = serde_json::from_str(body)?;
    let mut leaks = Vec::new();
    collect_json_path_leaks(artifact, "", &value, &mut leaks);
    assert_eq!(leaks, Vec::<String>::new());
    Ok(())
}

fn assert_ndjson_portable(artifact: &str, body: &str) -> TestResult {
    let mut leaks = Vec::new();
    for (index, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        collect_json_path_leaks(artifact, &format!("line{}", index + 1), &value, &mut leaks);
    }
    assert_eq!(leaks, Vec::<String>::new());
    Ok(())
}

#[test]
fn all_x06_json_proofs_do_not_leak_machine_paths() -> TestResult {
    let proof_dir = workspace_root().join("proof/memory");
    let mut checked = Vec::new();
    for entry in std::fs::read_dir(&proof_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("x06-") || !file_name.ends_with(".json") {
            continue;
        }
        let artifact = format!("proof/memory/{file_name}");
        let body = std::fs::read_to_string(&path)?;
        assert_json_portable(&artifact, &body)?;
        checked.push(file_name.to_owned());
    }
    checked.sort();
    assert!(
        checked.len() >= 50,
        "expected broad X06 proof coverage, checked only {checked:?}"
    );
    Ok(())
}

#[test]
fn x06_dogfood_and_learning_proofs_do_not_leak_machine_paths() -> TestResult {
    assert_json_portable(
        "proof/memory/x06-dogfood.json",
        include_str!("../../../proof/memory/x06-dogfood.json"),
    )?;
    assert_json_portable(
        "proof/memory/x06-learning-curve.json",
        include_str!("../../../proof/memory/x06-learning-curve.json"),
    )?;
    Ok(())
}

#[test]
fn x06_rag_rollup_proofs_do_not_leak_machine_paths() -> TestResult {
    for (artifact, body) in [
        (
            "proof/memory/x06-feature-parity.json",
            include_str!("../../../proof/memory/x06-feature-parity.json"),
        ),
        (
            "proof/memory/x06-rag-qa.json",
            include_str!("../../../proof/memory/x06-rag-qa.json"),
        ),
        (
            "proof/memory/x06-rag.json",
            include_str!("../../../proof/memory/x06-rag.json"),
        ),
        (
            "proof/memory/x06-retrieval-quality.json",
            include_str!("../../../proof/memory/x06-retrieval-quality.json"),
        ),
        (
            "proof/memory/x06-token-reduction.json",
            include_str!("../../../proof/memory/x06-token-reduction.json"),
        ),
    ] {
        assert_json_portable(artifact, body)?;
    }
    Ok(())
}

#[test]
fn x06_parity_trace_log_does_not_leak_machine_paths() -> TestResult {
    assert_ndjson_portable(
        "proof/memory/x06-parity/tool-results.ndjson",
        include_str!("../../../proof/memory/x06-parity/tool-results.ndjson"),
    )
}
