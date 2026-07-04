pub(crate) fn is_secret_like(text: &str) -> bool {
    let value = text.trim();
    github_token(value)
        || aws_key(value)
        || openai_key(value)
        || slack_or_package_key(value)
        || jwt_like_token(value)
        || pem_private_key(value)
        || stripe_live_key(value)
        || looks_high_entropy_secret(value)
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
    value.starts_with("sk-proj-") || value.starts_with("sk-") && value.len() > 24
}

fn slack_or_package_key(value: &str) -> bool {
    value.starts_with("xoxb-")
        || value.starts_with("xoxp-")
        || value.starts_with("pypi-")
        || value.starts_with("npm_")
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
