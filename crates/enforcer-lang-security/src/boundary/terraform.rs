//! Terraform/HCL source decoding for security validators.

use enforcer_domain::boundary::decode_error::DecodeError;
use regex::Regex;

/// One decoded Terraform resource block.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResourceBlock<'a> {
    pub(crate) resource_type: &'a str,
    pub(crate) name: &'a str,
    pub(crate) body: &'a str,
    pub(crate) line: u32,
}

/// Split source into brace-balanced Terraform resource blocks.
pub(crate) fn resource_blocks(source: &str) -> Vec<ResourceBlock<'_>> {
    let Ok(header) = Regex::new(r#"resource\s+"([A-Za-z0-9_]+)"\s+"([A-Za-z0-9_-]+)"\s*\{"#) else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    for capture in header.captures_iter(source) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        let open_brace = whole.end() - 1;
        let Some(close_brace) = matching_brace(source, open_brace) else {
            continue;
        };
        let Some(prefix) = source.get(..whole.start()) else {
            continue;
        };
        let line = u32::try_from(prefix.matches('\n').count())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        let Some(body) = source.get(open_brace.saturating_add(1)..close_brace) else {
            continue;
        };
        blocks.push(ResourceBlock {
            resource_type: capture.get(1).map_or("", |matched| matched.as_str()),
            name: capture.get(2).map_or("", |matched| matched.as_str()),
            body,
            line,
        });
    }
    blocks
}

fn matching_brace(source: &str, open_brace: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    for (offset, byte) in bytes.iter().enumerate().skip(open_brace) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Compile one static Terraform detection expression.
pub(crate) fn compile_regex(pattern: &'static str) -> Result<Regex, DecodeError> {
    crate::boundary::regex::compile("cyberskillsIacRegex", pattern)
}

/// Split an IAM policy body into decoded Statement object chunks.
pub(crate) fn statement_blocks(body: &str) -> Vec<&str> {
    let Some(marker) = body.find("Statement") else {
        return vec![body];
    };
    let mut chunks = Vec::new();
    let mut cursor = marker;
    while let Some(remaining) = body.get(cursor..) {
        let Some(relative_open) = remaining.find('{') else {
            break;
        };
        let open = cursor + relative_open;
        let Some(close) = matching_brace(body, open) else {
            break;
        };
        let Some(chunk) = body.get(open..=close) else {
            break;
        };
        chunks.push(chunk);
        let Some(next_cursor) = close.checked_add(1) else {
            break;
        };
        cursor = next_cursor;
        if cursor >= body.len() {
            break;
        }
    }
    if chunks.is_empty() {
        vec![body]
    } else {
        chunks
    }
}

/// Extract an integer HCL attribute.
pub(crate) fn int_attr(body: &str, name: &str) -> Option<i64> {
    body.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if !key.trim().eq_ignore_ascii_case(name) {
            return None;
        }
        value
            .trim()
            .trim_matches('"')
            .trim_end_matches([',', '}'])
            .trim()
            .parse::<i64>()
            .ok()
    })
}

pub(crate) fn bool_attr(body: &str, name: &str) -> Option<bool> {
    let pattern = Regex::new(&format!(r"(?i)\b{name}\s*=\s*(true|false)\b")).ok()?;
    let value = pattern.captures(body)?.get(1)?.as_str();
    Some(value.eq_ignore_ascii_case("true"))
}

pub(crate) fn list_attr_contains(body: &str, name: &str, value: &str) -> bool {
    let Ok(pattern) = Regex::new(&format!(r"(?is)\b{name}\s*=\s*\[([^]]*)]")) else {
        return false;
    };
    let quoted = format!("\"{value}\"");
    pattern
        .captures(body)
        .and_then(|capture| capture.get(1))
        .is_some_and(|matched| matched.as_str().contains(quoted.as_str()))
}

pub(crate) fn grants_public_access(body: &str) -> bool {
    string_attr_eq(body, "member", "allUsers")
        || string_attr_eq(body, "member", "allAuthenticatedUsers")
        || list_attr_contains(body, "members", "allUsers")
        || list_attr_contains(body, "members", "allAuthenticatedUsers")
}

pub(crate) fn allows_public_cidr(body: &str) -> bool {
    body.contains("0.0.0.0/0")
}

pub(crate) fn string_attr_eq(body: &str, name: &str, value: &str) -> bool {
    let Ok(pattern) = Regex::new(&format!(r#"(?i)\b{name}\s*=\s*"([^"]*)""#)) else {
        return false;
    };
    pattern
        .captures(body)
        .and_then(|capture| capture.get(1))
        .is_some_and(|matched| matched.as_str().eq_ignore_ascii_case(value))
}

pub(crate) fn covers_port_22(body: &str) -> bool {
    match (int_attr(body, "from_port"), int_attr(body, "to_port")) {
        (Some(from), Some(to)) => from <= 22 && to >= 22,
        _ => false,
    }
}

pub(crate) fn exposed_sensitive_port(body: &str) -> Option<&'static str> {
    let from = int_attr(body, "from_port")?;
    let to = int_attr(body, "to_port")?;
    if from <= 0 && to >= 65_535 {
        return Some("all ports (0.0.0.0/0:0-65535)");
    }
    [
        (3389, "RDP"),
        (3306, "MySQL"),
        (5432, "PostgreSQL"),
        (6379, "Redis"),
        (27017, "MongoDB"),
        (1433, "MSSQL"),
        (9200, "Elasticsearch"),
        (23, "Telnet"),
    ]
    .into_iter()
    .find_map(|(port, label)| (from <= port && to >= port).then_some(label))
}

pub(crate) fn ingress_subblocks(body: &str) -> Vec<&str> {
    let Ok(header) = Regex::new(r"ingress\s*\{") else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    for capture in header.captures_iter(body) {
        let Some(whole) = capture.get(0) else {
            continue;
        };
        let open_brace = whole.end() - 1;
        if let Some(close_brace) = matching_brace(body, open_brace) {
            if let Some(block) = body.get(open_brace.saturating_add(1)..close_brace) {
                blocks.push(block);
            }
        }
    }
    blocks
}
