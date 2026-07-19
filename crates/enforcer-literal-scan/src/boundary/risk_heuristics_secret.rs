pub(crate) fn is_secret_like(text: &str) -> bool {
    let value = text.trim();

    // Explicit token/key-pattern detectors must ALWAYS run, regardless of any
    // FP-suppression heuristic below. A real secret embedded in something
    // that merely *looks* like a model id, env-style config name, proof
    // filename, or cache path is still a real secret. Suppressions below are
    // only permitted to bypass the generic entropy heuristic
    // (looks_high_entropy_secret), never these explicit pattern checks.
    if github_token(value)
        || aws_key(value)
        || openai_key(value)
        || slack_or_package_key(value)
        || jwt_like_token(value)
        || pem_private_key(value)
        || stripe_live_key(value)
    {
        return true;
    }

    if is_hf_model_id(value)
        || is_env_style_config_value(value)
        || is_runtime_proof_filename(value)
        || is_model_artifact_or_source_path(value)
        || is_document_anchor_reference(value)
        || is_hf_cache_path(value)
        || is_interpolated_template(value)
        || is_code_or_resource_reference(value)
        || is_deterministic_identifier(value)
        || is_structured_nonsecret_reference(value)
    {
        return false;
    }

    looks_high_entropy_secret(value)
}

fn is_interpolated_template(value: &str) -> bool {
    (value.contains("${") && value.contains('}'))
        || (value.contains("{{") && value.contains("}}"))
        || (value.contains('{') && value.contains('}'))
        || contains_shell_variable(value)
}

fn contains_shell_variable(value: &str) -> bool {
    value.char_indices().any(|(index, ch)| {
        if ch != '$' {
            return false;
        }
        value
            .get(index + ch.len_utf8()..)
            .and_then(|tail| tail.chars().next())
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
    })
}

fn is_code_or_resource_reference(value: &str) -> bool {
    if value.contains(' ') || value.is_empty() {
        return false;
    }
    let path_shaped = value.starts_with("./")
        || value.starts_with("../")
        || value.contains("/")
        || value.contains("\\")
        || value.contains("://");
    if !path_shaped {
        return false;
    }
    if !value.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '/' | '\\' | '_' | '-' | '.' | ':' | '@' | '$' | '{' | '}' | '~' | '#'
            )
    }) {
        return false;
    }
    value
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .all(|segment| !looks_opaque_segment(segment))
}

fn looks_opaque_segment(value: &str) -> bool {
    if value.len() < 24 {
        return false;
    }
    let classes = [
        value.chars().any(|ch| ch.is_ascii_lowercase()),
        value.chars().any(|ch| ch.is_ascii_uppercase()),
        value.chars().any(|ch| ch.is_ascii_digit()),
        value.chars().any(|ch| matches!(ch, '+' | '_' | '-' | '=')),
    ];
    classes.iter().filter(|present| **present).count() >= 3
}

fn is_deterministic_identifier(value: &str) -> bool {
    let base = value.split('.').next().unwrap_or(value);
    ["evt_", "node_", "run_", "task_"]
        .iter()
        .find_map(|prefix| base.strip_prefix(prefix))
        .is_some_and(|body| {
            body.len() >= 16
                && (body.chars().all(|ch| ch.is_ascii_hexdigit())
                    || (body
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                        && body.contains("000000000000")))
        })
}

fn is_structured_nonsecret_reference(value: &str) -> bool {
    let serialized = (value.starts_with('{') && value.ends_with('}'))
        || (value.starts_with('[') && value.ends_with(']'))
        || (value.starts_with("{\\\"") && value.ends_with('}'));
    if serialized {
        return true;
    }
    if value.starts_with("(?") || value.starts_with("^(") {
        return true;
    }
    if value.contains("::")
        && !value.contains(' ')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
    {
        return value.split("::").all(|segment| !segment.is_empty());
    }
    if value.starts_with("href=\\\"") && value.contains('#') {
        return true;
    }
    if value.contains('.')
        && !value.contains(' ')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return value
            .split('.')
            .all(|segment| !looks_opaque_segment(segment));
    }
    false
}

fn is_env_style_config_value(value: &str) -> bool {
    let uppercase_only = value
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    // Require at least two underscore-delimited segments so a bare
    // high-entropy base32-ish secret (all-uppercase digits/underscore, but a
    // single opaque blob) does not masquerade as a multi-word env var name.
    let multi_segment = value.matches('_').count() >= 2;
    if uppercase_only && multi_segment && value.len() >= 8 {
        return true;
    }
    let known_suffixes = [
        "PATH",
        "PATHS",
        "MODEL_ID",
        "REVISION",
        "REV",
        "SHA256",
        "SHA_256",
        "FILE",
        "CACHE",
        "CACHE_DIR",
        "MODEL",
        "HOST",
        "PORT",
        "DIR",
        "OUT",
        "PROOF",
        "PROOF_OUT",
        "TOKEN",
        "NETWORK",
        "ID",
    ];
    let ends_with_known_suffix = known_suffixes.iter().any(|suffix| value.ends_with(suffix));

    // Also require an underscore here: a bare opaque token that happens to
    // end in a short generic suffix like "ID" or "REV" (e.g. a base32-style
    // secret) must not be suppressed just by coincidence of its trailing
    // characters. Legitimate env-var names are always underscore-delimited.
    uppercase_only && multi_segment && value.len() >= 8 && ends_with_known_suffix
}

fn is_hf_model_id(value: &str) -> bool {
    if value.contains("://") || value.contains(' ') || value.is_empty() {
        return false;
    }

    let mut model_and_quant = value.split(':');
    let model = model_and_quant.next().unwrap_or_default();
    let quant = model_and_quant.next();
    if model_and_quant.next().is_some() {
        return false;
    }
    let model_is_repo_like = if split_model_repo(model) {
        true
    } else {
        return false;
    };
    if let Some(quant) = quant {
        if quant.is_empty() || !is_hf_fragment(quant) {
            return false;
        }
    }

    model_is_repo_like
}

fn split_model_repo(value: &str) -> bool {
    let mut parts = value.split('/');
    let owner = match parts.next() {
        Some(value) => value,
        None => return false,
    };
    let repo = match parts.next() {
        Some(value) => value,
        None => return false,
    };
    if parts.next().is_some() {
        return false;
    }
    if !is_hf_fragment(owner) || !is_hf_fragment(repo) {
        return false;
    }
    // A real HF owner/repo slug is a short human-chosen name. If either
    // fragment on its own already looks like a random high-entropy blob
    // (e.g. encoded secret material that happens to contain exactly one '/'),
    // refuse to classify this as a benign model id so the entropy detector
    // still gets a chance to flag it.
    if looks_high_entropy_secret(owner) || looks_high_entropy_secret(repo) {
        return false;
    }
    true
}

fn is_hf_fragment(value: &str) -> bool {
    if value.len() < 2 || value.len() > 128 {
        return false;
    }
    let mut has_alpha_num = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            has_alpha_num = true;
        } else if matches!(ch, '-' | '_' | '.') {
        } else {
            return false;
        }
    }
    has_alpha_num
}

pub(crate) fn looks_high_entropy_secret(text: &str) -> bool {
    if text.len() < 32 || text.contains(' ') {
        return false;
    }
    let classes = [
        text.chars().any(|c| c.is_ascii_lowercase()),
        text.chars().any(|c| c.is_ascii_uppercase()),
        text.chars().any(|c| c.is_ascii_digit()),
        text.chars()
            .any(|c| matches!(c, '+' | '/' | '_' | '-' | '=' | '.')),
    ];
    classes.iter().filter(|value| **value).count() >= 3
}

fn is_runtime_proof_filename(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if !lower.contains("proof") || lower.len() < 32 {
        return false;
    }

    if ![".json", ".jsonl", ".txt", ".md", ".log"]
        .iter()
        .any(|ext| lower.ends_with(ext))
    {
        return false;
    }

    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '_' | '-' | '.'))
}

fn is_model_artifact_or_source_path(value: &str) -> bool {
    if value.contains("://") || value.contains(' ') || value.is_empty() {
        return false;
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '_' | '-' | '.'))
    {
        return false;
    }

    let lower = value.to_ascii_lowercase();
    let known_suffixes = [
        ".gguf",
        ".onnx",
        ".onnx_data",
        ".onnx.data",
        ".exe",
        ".ps1",
        ".mjs",
        ".js",
        ".ts",
        ".rs",
        ".json",
        ".jsonl",
        ".txt",
        ".md",
        ".log",
        ".zip",
        ".tar.gz",
    ];
    known_suffixes.iter().any(|suffix| lower.ends_with(suffix))
}

fn is_document_anchor_reference(value: &str) -> bool {
    let Some((document, anchor)) = value.split_once('#') else {
        return false;
    };
    if document.is_empty() || anchor.is_empty() || !document.to_ascii_lowercase().ends_with(".md") {
        return false;
    }
    document
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '_' | '-' | '.'))
        && anchor
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn is_hf_cache_path(value: &str) -> bool {
    if !value.starts_with("hf/") {
        return false;
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.'))
    {
        return false;
    }
    // Reject if any individual '/'-delimited segment after the "hf/" prefix
    // looks like a random high-entropy blob rather than a legitimate cache
    // directory component (e.g. a base64url-style secret smuggled in as
    // "hf/<secret>").
    let Some(rest) = value.get(3..) else {
        return false;
    };
    if rest.split('/').any(looks_high_entropy_secret) {
        return false;
    }
    true
}

// A real credential can be embedded anywhere inside a larger literal (e.g. a
// path, URL, or log line string), not only at the very start of it. These
// detectors therefore scan for the known prefix at any position, not just
// `starts_with`, so a suppression predicate can never hide a smuggled
// explicit-pattern secret just by wrapping it in extra path/filename text.
fn github_token(value: &str) -> bool {
    ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"]
        .iter()
        .any(|prefix| find_prefixed_run(value, prefix).is_some_and(|run| run.len() > 24))
}

fn aws_key(value: &str) -> bool {
    // AWS access key ids are strictly uppercase letters and digits, so trim
    // the matched run down to that charset before measuring length: a
    // suffix like ".json" appended after a real key body must not defeat
    // detection just because the generic run-scanner also accepts '.'.
    find_prefixed_run(value, "AKIA").is_some_and(|run| {
        let key_body_len = run
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            .count();
        key_body_len >= 20
    })
}

fn openai_key(value: &str) -> bool {
    find_prefixed_run(value, "sk-proj-").is_some()
        || find_prefixed_run(value, "sk-").is_some_and(|run| run.len() > 24)
}

fn slack_or_package_key(value: &str) -> bool {
    find_prefixed_run(value, "xoxb-").is_some_and(|run| run.len() > 24)
        || find_prefixed_run(value, "xoxp-").is_some_and(|run| run.len() > 24)
        || find_prefixed_run(value, "pypi-").is_some_and(|run| run.len() > 24)
        || find_prefixed_run(value, "npm_").is_some_and(|run| run.len() > 24)
}

fn jwt_like_token(value: &str) -> bool {
    find_prefixed_run(value, "eyJ")
        .is_some_and(|run| run.matches('.').count() == 2 && run.len() > 40)
}

fn pem_private_key(value: &str) -> bool {
    value.contains("-----BEGIN") && value.contains("PRIVATE KEY-----")
}

fn stripe_live_key(value: &str) -> bool {
    find_prefixed_run(value, "sk_live_").is_some_and(|run| run.len() > 20)
        || find_prefixed_run(value, "pk_live_").is_some_and(|run| run.len() > 20)
}

/// Finds the first occurrence of `prefix` inside `value` and returns the
/// contiguous "token run" starting at that position: the prefix plus every
/// following character that is a plausible credential-body character
/// (alphanumeric, `-`, `_`, `.`, `+`, `/`, `=`). This lets prefix-anchored
/// detectors match a credential embedded mid-string (e.g. inside a path)
/// while still measuring length/charset against just the credential body,
/// not trailing unrelated text such as a `.json` file extension.
fn find_prefixed_run<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let start = value.find(prefix)?;
    let rest = value.get(start..)?;
    let end = rest
        .char_indices()
        .find(|(_, ch)| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '+' | '/' | '='))
        })
        .map(|(idx, _)| idx)
        .unwrap_or(rest.len());
    rest.get(..end)
}

#[cfg(test)]
mod tests {
    use super::is_secret_like;

    #[test]
    fn env_style_runtime_config_names_are_not_secret_like() {
        assert!(!is_secret_like("ENFORCER_X06_CHAT_MODEL_ID"));
        assert!(!is_secret_like("ENFORCER_X06_CHAT_MODEL_REVISION"));
        assert!(!is_secret_like("ENFORCER_X06_EMBEDDING_ONNX_FILE"));
        assert!(!is_secret_like("ENFORCER_X06_ALLOW_NETWORK"));
        assert!(!is_secret_like("ENFORCER_X06_MODEL_CACHE"));
        assert!(!is_secret_like("ENFORCER_X06_CHILD_ARTIFACT_SHA256"));
        assert!(!is_secret_like("ENFORCER_X06_RUN_ID"));
        assert!(!is_secret_like(
            "ENFORCER_X06_TARGET_CHAT_TOKENS_PER_SECOND_HIGH"
        ));
    }

    #[test]
    fn hugging_face_model_ids_are_not_secret_like() {
        assert!(!is_secret_like("Qwen/Qwen3-4B-GGUF:Q4_K_M"));
        assert!(!is_secret_like(
            "bartowski/google_gemma-3-4b-it-GGUF:Q4_K_M"
        ));
        assert!(!is_secret_like("Qwen/Qwen3-Embedding-0.6B"));
        assert!(!is_secret_like("onnx-community/Qwen3-Reranker-0.6B-ONNX"));
        assert!(!is_secret_like("deepreinforce-ai/Ornith-1.0-9B-GGUF"));
    }

    #[test]
    fn proof_filenames_are_not_secret_like() {
        assert!(!is_secret_like(
            "proof/memory/x06-model-runtime-proof-qwen-embedding-onnx-cache-3f4e5d8b2a9c1e7f0d2a8b4c6e1f3d5a7b9c0a2e4f6b8d0c1e2a4b6d8f0a2c4e6.json"
        ));
        assert!(!is_secret_like(
            "proof/memory/x06-model-runtime-probe.jsonl"
        ));
    }

    #[test]
    fn model_artifact_and_source_paths_are_not_secret_like() {
        assert!(!is_secret_like("google_gemma-3-4b-it-Q4_K_M.gguf"));
        assert!(!is_secret_like(
            "target\\debug\\examples\\x06_model_runtime_probe.exe"
        ));
        assert!(!is_secret_like("../examples/x06_model_runtime_probe.rs"));
        assert!(!is_secret_like("../scripts/x06-real-model-proof.ps1"));
        assert!(!is_secret_like("hf/Qwen--Qwen3-Embedding-0.6B-GGUF/main"));
    }

    #[test]
    fn secret_like_string_remains_detected() {
        let openai_style = ["sk", "-", "abcdefghijklmnopqrstuvwxyz"].concat();
        let jwt_style = [
            "eyJ",
            "0eXAiOiJKV1QiLC",
            ".",
            "hbGciOiJIUzI1NiJ9",
            ".",
            "signature",
        ]
        .concat();

        assert!(is_secret_like(&openai_style));
        assert!(is_secret_like(&jwt_style));
    }

    #[test]
    fn code_references_templates_and_deterministic_ids_are_not_secrets() {
        assert!(!is_secret_like("./components/ProjectOverviewWorkspace"));
        assert!(!is_secret_like("${RELEASE_BASE_URL}/v${VERSION}/${asset}"));
        assert!(!is_secret_like("evt_099919a29a5e422b839b850265207b1b"));
        assert!(!is_secret_like(
            "node_7450523d7490414f86992de67525c1c2.codex-a"
        ));
        assert!(!is_secret_like(
            "https://github.com/ocentra/enforcer/releases/download"
        ));
        assert!(!is_secret_like("securityPipeline.coverage.previousLinePct"));
        assert!(!is_secret_like(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#
        ));
        assert!(!is_secret_like(
            "tests/fixtures/parity/docs/SAMPLE.md#SAMPLE-ANCHOR"
        ));
        assert!(!is_secret_like(
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
        ));
        assert!(!is_secret_like(
            "enforcer-mcp-fingerprint-v1\\nbinary={binary}\\nversion={}\\nruleset={ruleset_render}"
        ));
        assert!(!is_secret_like("enforcer-v$Version-$Variant-$Triple.zip"));
        assert!(!is_secret_like("no_reexports::NoReexportsValidator"));
        assert!(!is_secret_like(
            r#"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}"#
        ));
        assert!(!is_secret_like(r#"href=\"docs/rules/SAMPLE.md#SAMPLE-1\""#));
        assert!(!is_secret_like("evt_compatibility0000000000000000000000"));
        assert!(!is_secret_like("scripts/test/portal-e2e-runner.test.mjs"));
        assert!(!is_secret_like("boundary/npm_wrapper.rs"));
        assert!(!is_secret_like("proof/INDEX.md#legacy-script-migration"));
        assert!(!is_secret_like(
            "docs/PROOF_SYSTEM_DESIGN.md#deterministic-migration-sequence"
        ));
    }

    #[test]
    fn explicit_secret_inside_template_remains_detected() {
        let value = format!("${{HOME}}/{}{}.json", "ghp_", "A".repeat(32));
        assert!(is_secret_like(&value));
    }

    #[test]
    fn package_token_prefixes_require_a_credential_sized_body() {
        assert!(!is_secret_like("boundary/npm_wrapper.rs"));
        assert!(is_secret_like(
            &["npm_", "abcdefghijklmnopqrstuvwxyz", "012345"].concat()
        ));
        assert!(is_secret_like(
            &["pypi-", "abcdefghijklmnopqrstuvwxyz", "012345"].concat()
        ));
        assert!(is_secret_like(
            &["xoxb-", "abcdefghijklmnopqrstuvwxyz", "012345"].concat()
        ));
    }

    // --- Adversarial: explicit-prefix secrets smuggled behind a
    // suppression-shaped wrapper must still be detected. ---

    #[test]
    fn github_token_disguised_as_proof_filename_is_still_secret_like() {
        let value = format!("proof/memory/{}{}.json", "ghp_", "A".repeat(32));
        assert!(value.to_ascii_lowercase().contains("proof"));
        assert!(is_secret_like(&value));
    }

    #[test]
    fn aws_key_disguised_as_model_artifact_path_is_still_secret_like() {
        let value = ["AKIA", "ABCDEFGHIJKLMNOP", ".json"].concat();
        assert!(is_secret_like(&value));
    }

    #[test]
    fn github_token_disguised_as_model_artifact_path_is_still_secret_like() {
        let value = format!("{}{}.txt", "ghp_", "A".repeat(32));
        assert!(is_secret_like(&value));
    }

    #[test]
    fn openai_key_embedded_in_log_line_literal_is_still_secret_like() {
        // A real "sk-" key embedded inside a larger literal (e.g. a log
        // message string) must still be detected, not just when the whole
        // trimmed literal equals the key.
        let value = [
            "auth header: ",
            "sk-",
            "abcdefghijklmnopqrstuvwxyz",
            " sent",
        ]
        .concat();
        assert!(is_secret_like(&value));
    }

    #[test]
    fn pem_private_key_disguised_as_runtime_proof_filename_is_still_secret_like() {
        let value = ["-----BEGIN RSA ", "PRIVATE KEY----- proof.json"].concat();
        assert!(is_secret_like(&value));
    }

    // --- Adversarial: opaque high-entropy blobs that coincidentally match a
    // suppression's shape must not be suppressed. ---

    #[test]
    fn uppercase_underscore_high_entropy_blob_is_not_suppressed_as_env_style() {
        // All-uppercase/digits/underscore, single opaque segment (no
        // multi-word env-var structure): must not be treated as a benign
        // config name.
        let value = "ABCD1234EFGH5678IJKL9012MNOP_TOKEN";
        // Only one underscore -> should not qualify as multi-segment env var.
        assert_eq!(value.matches('_').count(), 1);
        assert!(is_secret_like(value));
    }

    #[test]
    fn opaque_blob_with_generic_short_suffix_is_not_suppressed_as_env_style() {
        // Single-underscore opaque blob ending in a short generic suffix
        // ("ID"): must not be suppressed merely because of the trailing
        // token combined with an all-uppercase charset.
        let value = "XQ7K2M9PZR4T8VBN3JH6DL1SWY0FGCEA_ID";
        assert_eq!(value.matches('_').count(), 1);
        assert!(is_secret_like(value));
    }

    #[test]
    fn base64_secret_with_single_slash_is_not_suppressed_as_hf_model_id() {
        // Exactly one '/', alnum + '-_.' only on each side: shape matches
        // owner/repo, but each fragment alone is a high-entropy blob.
        let secret = [
            "aZ3xK9mQ2vT7nR4",
            "wY8uL1oJ6hN0bCzA3dF5gH",
            "/",
            "pS7eI2rT9wQ4uL8",
            "oJ1hN6bC",
        ]
        .concat();
        assert_eq!(secret.matches('/').count(), 1);
        assert!(is_secret_like(&secret));
    }

    #[test]
    fn base64url_secret_under_hf_prefix_is_not_suppressed_as_cache_path() {
        let value = "hf/xK3n9dP2mQ7vT1zR8wY5uL0oJ6hN4bC-eA1fG3hJ5kL7";
        assert!(value.starts_with("hf/"));
        assert!(is_secret_like(value));
    }

    #[test]
    fn base64url_secret_multi_segment_under_hf_prefix_is_not_suppressed() {
        // Each individual segment (>=32 chars) is independently high-entropy,
        // so the per-segment guard in is_hf_cache_path must catch it even
        // though no single segment spans the whole value.
        let value = "hf/aZ3xK9mQ2vT7nR4wY8uL1oJ6hN0bCzA3d/F5gH9pS7eI2rT4wQ8uL1oJ6hN0bCzA3dF5g";
        assert!(is_secret_like(value));
    }

    // --- Regression: legitimate FP-suppression fixtures must stay green
    // after the reordering/tightening above. ---

    #[test]
    fn legitimate_suppressions_remain_non_secret_after_hardening() {
        assert!(!is_secret_like("ENFORCER_X06_CHAT_MODEL_ID"));
        assert!(!is_secret_like("ENFORCER_X06_CHILD_ARTIFACT_SHA256"));
        assert!(!is_secret_like(
            "ENFORCER_X06_TARGET_CHAT_TOKENS_PER_SECOND_HIGH"
        ));
        assert!(!is_secret_like("Qwen/Qwen3-4B-GGUF:Q4_K_M"));
        assert!(!is_secret_like(
            "bartowski/google_gemma-3-4b-it-GGUF:Q4_K_M"
        ));
        assert!(!is_secret_like("hf/Qwen--Qwen3-Embedding-0.6B-GGUF/main"));
        assert!(!is_secret_like(
            "proof/memory/x06-model-runtime-proof-qwen-embedding-onnx-cache-3f4e5d8b2a9c1e7f0d2a8b4c6e1f3d5a7b9c0a2e4f6b8d0c1e2a4b6d8f0a2c4e6.json"
        ));
        assert!(!is_secret_like("google_gemma-3-4b-it-Q4_K_M.gguf"));
    }
}
