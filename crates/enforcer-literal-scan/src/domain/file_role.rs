use crate::{FileRole, LanguageFamily, LanguageSpec};
use enforcer_domain::scan_types::LiteralFindingPath;

pub(crate) fn classify_file_role(rel: &LiteralFindingPath, language: LanguageSpec) -> FileRole {
    let lower = rel.as_str().to_ascii_lowercase();
    if language.family == LanguageFamily::CommonText {
        if lower.ends_with(".md") || lower.ends_with(".mdx") || lower.ends_with(".txt") {
            return FileRole::Docs;
        }
        return FileRole::CommonText;
    }
    if lower.split('/').any(|part| part == "generated")
        || lower.split('/').any(|part| part == "__generated__")
        || lower.contains("auto-generated")
    {
        return FileRole::Generated;
    }
    if lower.split('/').any(|part| part == "test")
        || lower.split('/').any(|part| part == "tests")
        || lower.split('/').any(|part| part == "__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.contains("/test_")
    {
        return FileRole::Test;
    }
    if lower.split('/').any(|part| part == "boundary")
        || lower.split('/').any(|part| part == "boundaries")
        || lower.split('/').any(|part| part == "adapter")
        || lower.split('/').any(|part| part == "adapters")
        || lower.split('/').any(|part| part == "transport")
        || lower.split('/').any(|part| part == "serde")
        || lower.split('/').any(|part| part == "dto")
        || lower.split('/').any(|part| part == "request")
        || lower.split('/').any(|part| part == "response")
    {
        return FileRole::Boundary;
    }
    if lower.split('/').any(|part| part == "config")
        || lower.split('/').any(|part| part == "settings")
        || lower.split('/').any(|part| part == "env")
    {
        return FileRole::Config;
    }
    if lower.split('/').any(|part| part == "scripts")
        || lower.split('/').any(|part| part == "tools")
        || language.family == LanguageFamily::Shell
    {
        return FileRole::Script;
    }
    if lower.split('/').any(|part| part == "domain")
        || lower.split('/').any(|part| part == "domains")
        || lower.split('/').any(|part| part == "core")
        || lower.split('/').any(|part| part == "model")
        || lower.split('/').any(|part| part == "models")
    {
        return FileRole::Domain;
    }
    FileRole::Unknown
}
