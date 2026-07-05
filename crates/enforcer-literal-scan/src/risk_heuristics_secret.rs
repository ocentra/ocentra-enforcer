pub(crate) fn is_secret_like(text: &str) -> bool {
    let value = text.trim();
    if is_hf_model_id(value)
        || is_env_style_config_value(value)
        || is_runtime_proof_filename(value)
        || is_model_artifact_or_source_path(value)
        || is_hf_cache_path(value)
    {
        return false;
    }
    github_token(value)
        || aws_key(value)
        || openai_key(value)
        || slack_or_package_key(value)
        || jwt_like_token(value)
        || pem_private_key(value)
        || stripe_live_key(value)
        || looks_high_entropy_secret(value)
}

fn is_env_style_config_value(value: &str) -> bool {
    let uppercase_only = value
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    if uppercase_only && value.contains('_') && value.len() >= 8 {
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

    uppercase_only && value.len() >= 8 && ends_with_known_suffix
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
        ".rs",
        ".json",
        ".jsonl",
        ".txt",
        ".md",
        ".log",
    ];
    known_suffixes.iter().any(|suffix| lower.ends_with(suffix))
}

fn is_hf_cache_path(value: &str) -> bool {
    if !value.starts_with("hf/") {
        return false;
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.'))
}

fn github_token(value: &str) -> bool {
    (value.starts_with("ghp_")
        || value.starts_with("gho_")
        || value.starts_with("ghu_")
        || value.starts_with("ghs_")
        || value.starts_with("ghr_"))
        && value.len() > 24
}

fn aws_key(value: &str) -> bool {
    value.starts_with("AKIA")
        && value.len() >= 20
        && value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

fn openai_key(value: &str) -> bool {
    starts_with_parts(value, &["sk", "-", "proj", "-"])
        || starts_with_parts(value, &["sk", "-"]) && value.len() > 24
}

fn slack_or_package_key(value: &str) -> bool {
    starts_with_parts(value, &["xoxb", "-"])
        || starts_with_parts(value, &["xoxp", "-"])
        || starts_with_parts(value, &["pypi", "-"])
        || starts_with_parts(value, &["npm", "_"])
}

fn jwt_like_token(value: &str) -> bool {
    value.starts_with("eyJ") && value.matches('.').count() == 2 && value.len() > 40
}

fn pem_private_key(value: &str) -> bool {
    value.contains("-----BEGIN") && value.contains("PRIVATE KEY-----")
}

fn stripe_live_key(value: &str) -> bool {
    (value.starts_with("sk_live_") || value.starts_with("pk_live_")) && value.len() > 20
}

fn starts_with_parts(value: &str, parts: &[&str]) -> bool {
    let mut expected = String::new();
    for part in parts {
        expected.push_str(part);
    }
    value.starts_with(&expected)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
