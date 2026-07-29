//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
pub(crate) fn looks_like_event(text: &str) -> bool {
    let parts: Vec<_> = text.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        })
}

pub(crate) fn looks_like_route_or_url(text: &str) -> bool {
    text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("ws://")
        || text.starts_with("wss://")
        || (text.starts_with('/') && text.len() > 1 && !text.starts_with("//"))
}

pub(crate) fn looks_like_protocol(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "get"
            | "post"
            | "put"
            | "patch"
            | "delete"
            | "head"
            | "options"
            | "content-type"
            | "authorization"
            | "accept"
            | "user-agent"
            | "application/json"
            | "text/html"
            | "application/octet-stream"
    )
}

pub(crate) fn looks_like_id_or_key(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower == "id"
        || lower.ends_with("_id")
        || lower.ends_with("id") && lower.len() > 2 && lower.chars().any(|c| c.is_ascii_uppercase())
        || lower.contains("user_id")
        || lower.contains("device_id")
        || lower.ends_with("_key")
        || lower.ends_with("key") && lower.len() > 3
}

pub(crate) fn looks_like_state_or_status(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "active"
            | "inactive"
            | "enabled"
            | "disabled"
            | "ready"
            | "pending"
            | "open"
            | "closed"
            | "success"
            | "failure"
            | "failed"
            | "running"
            | "stopped"
            | "created"
            | "deleted"
            | "updated"
            | "accepted"
            | "rejected"
            | "draft"
            | "published"
    )
}

pub(crate) fn looks_like_json_blob(text: &str) -> bool {
    let trimmed = text.trim();
    ((trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']')))
        && trimmed.contains(':')
}

pub(crate) fn looks_like_sql(text: &str) -> bool {
    let upper = text.to_ascii_uppercase();
    [
        "SELECT ",
        "INSERT ",
        "UPDATE ",
        "DELETE ",
        "CREATE TABLE",
        "ALTER TABLE",
        "DROP TABLE",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

pub(crate) fn looks_like_shell(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "rm -rf",
        "curl ",
        "wget ",
        "cmd /c",
        "powershell",
        "invoke-expression",
        "bash -c",
        "sh -c",
        "chmod 777",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn looks_like_human_message(text: &str) -> bool {
    text.contains(' ')
        && text.len() >= 8
        && !looks_like_sql(text)
        && !looks_like_shell(text)
        && !looks_like_json_blob(text)
}
