//! No hardcoded provider credentials: every secret is read from the
//! environment / a secret store at runtime, never a literal in source.

fn aws_key() -> Result<String, std::env::VarError> {
    std::env::var("AWS_ACCESS_KEY_ID")
}

fn github_token() -> Result<String, std::env::VarError> {
    std::env::var("GITHUB_TOKEN")
}

fn stripe_key() -> Result<String, std::env::VarError> {
    std::env::var("STRIPE_SECRET_KEY")
}
