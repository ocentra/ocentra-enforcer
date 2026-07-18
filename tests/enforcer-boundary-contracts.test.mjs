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
