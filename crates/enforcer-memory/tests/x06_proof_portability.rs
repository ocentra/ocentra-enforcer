type TestResult = Result<(), Box<dyn std::error::Error>>;

fn looks_like_machine_absolute_path(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    for index in 0..chars.len().saturating_sub(3) {
        let drive = chars[index];
        if !drive.is_ascii_alphabetic() || chars[index + 1] != ':' {
            continue;
        }

        let separator = chars[index + 2];
        if separator != '\\' && separator != '/' {
            continue;
        }

        let previous = index
            .checked_sub(1)
            .map(|previous_index| chars[previous_index]);
        if previous.is_some_and(|character| character.is_ascii_alphanumeric()) {
            continue;
        }

        let next = chars[index + 3];
        if next.is_ascii_alphanumeric() || matches!(next, '.' | '_' | '-' | '\\' | '/') {
            return true;
        }
    }

    false
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
fn x06_parity_trace_log_does_not_leak_machine_paths() -> TestResult {
    assert_ndjson_portable(
        "proof/memory/x06-parity/tool-results.ndjson",
        include_str!("../../../proof/memory/x06-parity/tool-results.ndjson"),
    )
}
