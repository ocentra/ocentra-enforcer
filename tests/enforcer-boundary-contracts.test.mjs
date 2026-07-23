import assert from "node:assert/strict";
import test from "node:test";
import { makeProject, runGateArgs } from "./rust-rules-fixture.mjs";

function commonBoundaryScan(project, files) {
  return runGateArgs(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    ...files,
  ]);
}

function ruleIds(result) {
  return new Set(JSON.parse(result.stdout).violations.map((violation) => violation.ruleId));
}

test("BOUND rules reject raw DTO leakage and missing domain conversion", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: wire text is decoded before domain use.
pub struct RawUserDto { pub value: String }

pub fn decode(_input: &str) -> RawUserDto {
    RawUserDto { value: String::new() }
}
`,
    "tests/wire.rs": `
#[test]
fn malformed_wire_input_is_rejected() {
    assert!("{".parse::<usize>().is_err());
}
`,
  });
  const result = commonBoundaryScan(project, ["src/boundary/wire.rs", "tests/wire.rs"]);
  const ids = ruleIds(result);
  assert.equal(ids.has("BOUND-1.2"), true);
  assert.equal(ids.has("BOUND-1.9"), true);
});

test("BOUND-1.10 rejects primitive Rust error channels", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: raw users convert to validated domain identifiers.
pub struct RawUserDto { pub value: String }
pub struct UserId(String);

pub fn convert(raw: RawUserDto) -> Result<UserId, String> {
    if raw.value.is_empty() { return Err("empty".to_owned()); }
    Ok(UserId(raw.value))
}
`,
    "tests/wire.rs": `
#[test]
fn malformed_raw_user_is_rejected() {
    let raw = fixture::boundary::wire::RawUserDto { value: String::new() };
    assert!(fixture::boundary::wire::convert(raw).is_err());
}
`,
  });
  const result = commonBoundaryScan(project, ["src/boundary/wire.rs", "tests/wire.rs"]);
  assert.equal(ruleIds(result).has("BOUND-1.10"), true);
});

test("typed boundary conversion stays inside the DTO seam", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: raw users convert to validated domain identifiers.
pub struct RawUserDto { pub value: String }
pub struct UserId(String);
pub enum DecodeError { Empty }

pub fn convert(raw: RawUserDto) -> Result<UserId, DecodeError> {
    if raw.value.is_empty() { return Err(DecodeError::Empty); }
    Ok(UserId(raw.value))
}
`,
    "tests/wire.rs": `
#[test]
fn malformed_raw_user_is_rejected() {
    let raw = fixture::boundary::wire::RawUserDto { value: String::new() };
    assert!(fixture::boundary::wire::convert(raw).is_err());
}
`,
  });
  const result = commonBoundaryScan(project, ["src/boundary/wire.rs", "tests/wire.rs"]);
  const ids = ruleIds(result);
  for (const ruleId of ["BOUND-1.2", "BOUND-1.9", "BOUND-1.10"]) {
    assert.equal(ids.has(ruleId), false, `${ruleId} must pass for a typed conversion`);
  }
});

test("crate-private DTO seams and typed unit validators do not leak boundary values", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: raw users convert to validated domain identifiers.
pub struct RawUserDto { pub value: String }
pub struct UserId(String);
pub enum DecodeError { Empty }

impl From<RawUserDto> for UserId {
    fn from(raw: RawUserDto) -> Self { Self(raw.value) }
}

pub(crate) fn decode(raw: RawUserDto) -> Result<RawUserDto, DecodeError> { Ok(raw) }
pub(crate) fn validate_user(_user: &UserId) -> Result<(), DecodeError> { Ok(()) }
`,
    "tests/wire.rs": `
#[test]
fn malformed_raw_user_is_rejected() {
    let raw = fixture::boundary::wire::RawUserDto { value: String::new() };
    assert!(fixture::boundary::wire::decode(raw).is_err());
}
`,
  });
  const result = commonBoundaryScan(project, ["src/boundary/wire.rs", "tests/wire.rs"]);
  const ids = ruleIds(result);
  for (const ruleId of ["BOUND-1.2", "BOUND-1.9", "BOUND-1.10"]) {
    assert.equal(ids.has(ruleId), false, `${ruleId} must allow a crate-private typed seam`);
  }
});

test("documented serialized output DTOs remain valid at a public boundary edge", () => {
  const project = makeProject({
    "src/boundary/output.rs": `
//! BOUNDARY-INVARIANT: validated domain data is serialized once at the API edge.
//! ROUNDTRIP-TEST: tests/output.rs::response_round_trip_preserves_fields
#[derive(Serialize)]
pub struct UserResponseDto { pub id: String }
pub fn render_user() -> UserResponseDto { UserResponseDto { id: String::new() } }
`,
    "tests/output.rs": "#[test] fn response_round_trip_preserves_fields() {}",
  });
  const result = commonBoundaryScan(project, ["src/boundary/output.rs", "tests/output.rs"]);
  assert.equal(ruleIds(result).has("BOUND-1.9"), false);
});

test("BOUND scanner ignores unknown_variant helper names in error mapping", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: wire tags are decoded into domain values.
use enforcer_domain::memory_types::MemoryBundleSchemaVersion;
use serde::{Deserialize, Deserializer};

pub enum KnownKind {
  Unknown,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum KnownKindWire {
  Unknown,
  Known,
}

pub(crate) fn deserialize_kind<'de, D>(deserializer: D) -> Result<MemoryBundleSchemaVersion, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer)?.as_str() {
        "known" => Ok(MemoryBundleSchemaVersion::INITIAL),
        value => Err(serde::de::Error::unknown_variant(value, &["known"])),
    }
}
`,
    "tests/wire.rs": `
#[test]
fn known_variant_round_trip() {
    assert_eq!(1, 1);
}
`,
  });
  const result = commonBoundaryScan(project, ["src/boundary/wire.rs", "tests/wire.rs"]);
  const ids = ruleIds(result);
  assert.equal(ids.has("BOUND-1.2"), false);
});

test("BOUND signature leak detection ignores boundary words in comments only", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: parse raw wire DTOs before calling domain logic.
//! Example DTO: UserResponseDto and LegacyRequest must stay internal.
use enforcer_domain::memory_types::MemoryBundleSchemaVersion;

pub fn decode_kind(raw: &str) -> usize {
    if raw.trim().is_empty() {
        return 0;
    }
    raw.len()
}
`,
    "tests/wire.rs": `
#[test]
fn decode_kind_lengths() {
    assert_eq!(decode_kind("abc"), 3);
}
`,
  });
  const result = commonBoundaryScan(project, ["src/boundary/wire.rs", "tests/wire.rs"]);
  const ids = ruleIds(result);
  assert.equal(ids.has("BOUND-1.9"), false);
});
