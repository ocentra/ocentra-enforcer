use super::{FileRole, LanguageFamily, LanguageSpec};
pub(crate) fn classify_file_role(rel: &str, language: LanguageSpec) -> FileRole {
    let lower = rel.to_ascii_lowercase();
    if language.family == LanguageFamily::CommonText {
        if lower.ends_with(".md") || lower.ends_with(".mdx") || lower.ends_with(".txt") {
            return FileRole::Docs;
        }
        return FileRole::CommonText;
    }
    if contains_segment(&lower, "generated")
        || contains_segment(&lower, "__generated__")
        || lower.contains("auto-generated")
    {
        return FileRole::Generated;
    }
    if contains_segment(&lower, "test")
        || contains_segment(&lower, "tests")
        || contains_segment(&lower, "__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.contains("/test_")
    {
        return FileRole::Test;
    }
    if contains_segment(&lower, "boundary")
        || contains_segment(&lower, "boundaries")
        || contains_segment(&lower, "adapter")
        || contains_segment(&lower, "adapters")
        || contains_segment(&lower, "transport")
        || contains_segment(&lower, "serde")
        || contains_segment(&lower, "ffi")
        || contains_segment(&lower, "dto")
        || contains_segment(&lower, "request")
        || contains_segment(&lower, "response")
    {
        return FileRole::Boundary;
    }
    if contains_segment(&lower, "config")
        || contains_segment(&lower, "settings")
        || contains_segment(&lower, "env")
    {
        return FileRole::Config;
    }
    if contains_segment(&lower, "scripts")
        || contains_segment(&lower, "tools")
        || language.family == LanguageFamily::Shell
    {
        return FileRole::Script;
    }
    if contains_segment(&lower, "domain")
        || contains_segment(&lower, "domains")
        || contains_segment(&lower, "core")
        || contains_segment(&lower, "model")
        || contains_segment(&lower, "models")
    {
        return FileRole::Domain;
    }
    FileRole::Unknown
}

fn contains_segment(path: &str, segment: &str) -> bool {
    path.split('/').any(|part| part == segment)
}
