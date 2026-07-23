//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Conversion of transport-owned CLI argument text into domain-owned values.

use crate::cli::CliError;
use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    MemoryCliArgsJson, MemoryCliArgument, MemoryCliArguments, MemoryCliEnvelopeText,
    MemoryCliFlagKey, MemoryCliFlagValue,
};
use serde_json::Value;

/// An external argument collection that can cross into the memory CLI domain.
pub trait CliBoundaryArguments {
    fn into_memory_cli_arguments(self) -> MemoryCliArguments;
}

impl<I, S> CliBoundaryArguments for I
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fn into_memory_cli_arguments(self) -> MemoryCliArguments {
        self.into_iter()
            .map(|argument| MemoryCliArgument::from(argument.as_ref()))
            .collect::<Vec<_>>()
            .into()
    }
}

/// Convert transport-owned flag tokens into the CLI's JSON argument object.
/// This parser remains at the raw JSON boundary; the CLI domain receives only
/// the branded serialized argument payload.
pub(crate) fn flags_to_json(tokens: &[&str]) -> Result<MemoryCliArgsJson, CliError> {
    let mut map: serde_json::Map<String, Value> = serde_json::Map::new();
    let mut tokens = tokens.iter().copied().peekable();
    while let Some(token) = tokens.next() {
        let Some(flag) = token.strip_prefix("--") else {
            return Err(CliError::InvalidJson(format!(
                "expected a --flag, got {token:?}"
            )));
        };
        let raw_key: MemoryCliFlagKey = flag.into();
        let key = kebab_to_key(&raw_key);
        let value: Value = match tokens.peek().copied() {
            Some(next) if !next.starts_with("--") => match tokens.next() {
                Some(raw) => parse_flag_value(&raw.into()),
                None => Value::Bool(true),
            },
            _ => Value::Bool(true),
        };
        insert_or_accumulate(&mut map, key, value);
    }
    serde_json::to_string(&Value::Object(map))
        .map(Into::into)
        .map_err(|source| CliError::InvalidJson(format!("failed to encode flags: {source}")))
}

fn insert_or_accumulate(
    map: &mut serde_json::Map<String, Value>,
    key: MemoryCliFlagKey,
    value: Value,
) {
    match map.get_mut(key.as_str()) {
        None => {
            map.insert(key.into(), value);
        }
        Some(Value::Array(existing)) => existing.push(value),
        Some(existing) => {
            let previous = existing.retained();
            map.insert(key.into(), Value::Array(vec![previous, value]));
        }
    }
}

fn parse_flag_value(raw: &MemoryCliFlagValue) -> Value {
    match raw.as_str() {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        other => other
            .parse::<i64>()
            .map(|n| Value::Number(n.into()))
            .unwrap_or_else(|_| Value::String(other.retained())),
    }
}

/// Convert one kebab-case transport flag name into the canonical camelCase
/// key expected by the MCP wire schema.
fn kebab_to_key(flag: &MemoryCliFlagKey) -> MemoryCliFlagKey {
    let mut out = String::with_capacity(flag.len());
    let mut uppercase_next = false;
    for c in flag.chars() {
        if c == '-' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            out.extend(c.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(c);
        }
    }
    out.into()
}

pub(crate) fn envelope_text(envelope: &Value) -> Option<MemoryCliEnvelopeText> {
    envelope
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|entry| entry.get("text"))
        .and_then(Value::as_str)
        .map(Into::into)
}
