//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Raw source-text predicates evaluated at the security scanning boundary.

use regex::Regex;

use crate::boundary::pattern::PatternConfidence;
use enforcer_domain::paths::RelPath;

pub(crate) fn path_is_forbidden(path: &RelPath, allowed: &[Regex], forbidden: &[Regex]) -> bool {
    let path = path.as_str();
    !allowed.iter().any(|pattern| pattern.is_match(path))
        && forbidden.iter().any(|pattern| pattern.is_match(path))
}

pub(crate) fn is_dynamic_argument(argument: &str, fstring_prefix: &Regex) -> bool {
    let trimmed = argument.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('+') || trimmed.contains("${") || trimmed.contains("#{") {
        return true;
    }
    if fstring_prefix.is_match(trimmed) {
        return true;
    }
    !trimmed.contains('"') && !trimmed.contains('\'') && !trimmed.contains('`')
}

pub(crate) fn shell_enabled_or_dynamic(value: &str) -> bool {
    !matches!(value.trim(), "False" | "None" | "0")
}

pub(crate) fn is_dynamic_template(
    argument: &str,
    fstring_prefix: &Regex,
    percent_format: &Regex,
) -> bool {
    let trimmed = argument.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('+') || trimmed.contains(".format(") {
        return true;
    }
    if trimmed.contains('`') && trimmed.contains("${") {
        return true;
    }
    if fstring_prefix.is_match(trimmed) || percent_format.is_match(trimmed) {
        return true;
    }
    !trimmed.contains('"') && !trimmed.contains('\'') && !trimmed.contains('`')
}

pub(crate) fn is_interpolated_template_literal(argument: &str) -> bool {
    let trimmed = argument.trim();
    trimmed.contains('`') && trimmed.contains("${")
}

pub(crate) fn is_explicit_disable(line: &str, start: usize) -> bool {
    matches!(
        line.get(..start).and_then(|prefix| prefix.chars().last()),
        Some('-') | Some('!')
    )
}

pub(crate) fn is_suspicious_codepoint(character: char) -> bool {
    matches!(
        u32::from(character),
        0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xE0000..=0xE007F
    )
}

pub(crate) fn suspicious_codepoint_label(character: char) -> &'static str {
    match u32::from(character) {
        0x200B..=0x200F => "a zero-width / directional-mark character (U+200B-U+200F)",
        0x202A..=0x202E => "a bidi-override character (U+202A-U+202E)",
        0x2060..=0x206F => "an invisible formatting character (U+2060-U+206F)",
        _ => "a Unicode TAG character (U+E0000-U+E007F)",
    }
}

pub(crate) fn modsec_label(line: &str) -> Option<&'static str> {
    const MODSEC_RULE_MAP: &[(&str, &str)] = &[
        ("942100", "SQL Injection via libinjection"),
        ("942110", "SQL Injection (common keywords)"),
        ("942120", "SQL Injection operator"),
        ("942130", "SQL Injection tautology"),
        ("942140", "SQL Injection (DB names)"),
        ("942150", "SQL Injection (functions)"),
        ("942160", "SQL Injection blind test (sleep/benchmark)"),
        ("942170", "SQL Injection (UNION query)"),
        ("942180", "SQL Injection bypass (basic auth)"),
        ("942190", "SQL Injection (MSSQL exec)"),
        ("942200", "SQL Injection (MySQL comment/space obfuscation)"),
        ("942210", "SQL Injection (chained)"),
        ("942220", "SQL Injection (integer overflow)"),
        ("942230", "SQL Injection (conditional)"),
        ("942240", "SQL Injection (MySQL charset switch)"),
        ("942250", "SQL Injection (MATCH AGAINST)"),
        ("942260", "SQL Injection bypass (basic auth 2)"),
        ("942270", "SQL Injection (common DB names)"),
        ("942280", "SQL Injection (pg_sleep/waitfor)"),
        ("942290", "SQL Injection (MongoDB)"),
    ];
    static ID_REGEX: std::sync::OnceLock<Result<Regex, regex::Error>> = std::sync::OnceLock::new();
    let Ok(id_regex) = ID_REGEX.get_or_init(|| Regex::new(r#"id:"?(\d{6})"?"#)) else {
        return None;
    };
    let id = id_regex.captures(line)?.get(1)?.as_str();
    MODSEC_RULE_MAP
        .iter()
        .find(|(rule_id, _)| *rule_id == id)
        .map(|(_, label)| *label)
}

pub(crate) fn severity_weight(confidence: PatternConfidence) -> u8 {
    match confidence {
        PatternConfidence::Critical => 3,
        PatternConfidence::High => 2,
        PatternConfidence::Medium => 1,
    }
}

pub(crate) fn redact_secret(matched: &str) -> String {
    let prefix: String = matched.chars().take(4).collect();
    format!("{prefix}<redacted>")
}
