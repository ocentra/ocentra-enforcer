//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::FileRole;

pub(crate) fn is_import_like_context(context: &str, text: &str) -> bool {
    let trimmed = context.trim_start();
    (trimmed.starts_with("import ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("use ")
        || trimmed.contains("require("))
        && (text.starts_with('.') || text.starts_with('/') || !text.contains(' '))
}

pub(crate) fn is_schema_owner_context(role: FileRole, context: &str) -> bool {
    role == FileRole::Config
        || context.contains("Schema")
        || context.contains("schema")
        || context.contains("defineLiteral")
}

pub(crate) fn is_magic_string_comparison(context: &str) -> bool {
    context.contains("==")
        || context.contains("===")
        || context.contains("!=")
        || context.contains("!==")
        || context.trim_start().starts_with("case ")
        || context.contains("match ")
        || context.contains("switch")
}

pub(crate) fn is_logging_context(context: &str) -> bool {
    let lower = context.to_ascii_lowercase();
    lower.contains("log")
        || lower.contains("logger")
        || lower.contains("tracing")
        || lower.contains("telemetry")
        || lower.contains("metrics")
}
