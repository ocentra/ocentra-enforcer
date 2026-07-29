import test from 'node:test';
import assert from 'node:assert/strict';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-domain-evidence-fixture.mjs';

test('RR-14.25 rejects a target namespace method returning an unrelated codec value', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto {
    pub value: String,
}

impl InputDto {
    pub fn unrelated_json() -> serde_json::Value {
        serde_json::json!({ "value": "" })
    }
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;

#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {
    let original = InputDto::unrelated_json();
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded: InputDto = serde_json::from_slice(&encoded).unwrap();
    assert!(original.is_object() && decoded.value.is_empty());
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-12.18 rejects keyword-only DTO conversion evidence', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto {
    pub value: String,
}

pub struct DomainValue;

impl TryFrom<InputDto> for DomainValue {
    type Error = ();

    fn try_from(_dto: InputDto) -> Result<Self, Self::Error> {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_payload_is_rejected() {}
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO conversion from InputDto to DomainValue lacks negative test evidence/u, output);
});

test('RR-12.18 rejects an unrelated conversion error assertion', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto {
    pub value: String,
}

pub struct DomainValue;

impl TryFrom<InputDto> for DomainValue {
    type Error = ();

    fn try_from(_dto: InputDto) -> Result<Self, Self::Error> {
        Err(())
    }
}

pub struct OtherDomain;

impl TryFrom<&str> for OtherDomain {
    type Error = ();

    fn try_from(_raw: &str) -> Result<Self, Self::Error> {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DomainValue, InputDto, OtherDomain};

    #[test]
    fn invalid_payload_is_rejected() {
        let _valid = DomainValue::try_from(InputDto { value: "valid".to_owned() });
        let unrelated = OtherDomain::try_from("bad");
        assert!(unrelated.is_err());
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO conversion from InputDto to DomainValue lacks negative test evidence/u, output);
});

test('RR-12.18 accepts typed DTO variable dataflow into an asserted conversion error', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto { pub value: String }
pub struct DomainValue;
impl TryFrom<InputDto> for DomainValue {
    type Error = ();
    fn try_from(_dto: InputDto) -> Result<Self, Self::Error> { Err(()) }
}
`,
    'tests/input_rejection.rs': `
use fixture::boundary::input::{DomainValue, InputDto};

#[test]
fn invalid_input_dto_is_rejected() {
    let invalid: InputDto = InputDto { value: String::new() };
    let result = DomainValue::try_from(invalid);
    assert!(result.is_err());
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /DTO conversion from InputDto to DomainValue lacks negative test evidence/u, output);
});

test('RR-12.18 follows an unannotated DTO variable only from a typed producer signature', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto { pub value: String }
pub struct DomainValue;
impl TryFrom<InputDto> for DomainValue {
    type Error = ();
    fn try_from(_dto: InputDto) -> Result<Self, Self::Error> { Err(()) }
}
`,
    'tests/input_rejection.rs': `
use fixture::boundary::input::{DomainValue, InputDto};

fn invalid_dto() -> Result<InputDto, ()> {
    Ok(InputDto { value: String::new() })
}

#[test]
fn invalid_produced_input_dto_is_rejected() {
    let invalid = invalid_dto().unwrap();
    let result = DomainValue::try_from(invalid);
    assert!(result.is_err());
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO conversion from InputDto to DomainValue lacks negative test evidence/u, output);
});

test('RR-12.18 rejects an unannotated producer returning a different DTO', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto { pub value: String }
pub struct OtherDto { pub value: String }
pub struct DomainValue;
impl TryFrom<InputDto> for DomainValue {
    type Error = ();
    fn try_from(_dto: InputDto) -> Result<Self, Self::Error> { Err(()) }
}
`,
    'tests/input_rejection.rs': `
use fixture::boundary::input::{DomainValue, OtherDto};

fn invalid_dto() -> Result<OtherDto, ()> {
    Ok(OtherDto { value: String::new() })
}

#[test]
fn unrelated_produced_dto_is_not_evidence() {
    let invalid = invalid_dto().unwrap();
    let result = DomainValue::try_from(invalid);
    assert!(result.is_err());
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO conversion from InputDto to DomainValue lacks negative test evidence/u, output);
});

test('RR-12.18 accepts a brace-balanced inline DTO conversion rejection', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto { pub value: String }
pub struct DomainValue;
impl TryFrom<InputDto> for DomainValue {
    type Error = ();
    fn try_from(_dto: InputDto) -> Result<Self, Self::Error> { Err(()) }
}
`,
    'tests/input_rejection.rs': `
use fixture::boundary::input::{DomainValue, InputDto};

#[test]
fn malformed_inline_input_dto_is_rejected() {
    assert!(DomainValue::try_from(InputDto { value: String::new() }).is_err());
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /DTO conversion from InputDto to DomainValue lacks negative test evidence/u, output);
});

test('parser rejection evidence accepts an observed Err match on the exact result', () => {
  const project = makeProject({
    'src/parser.rs': `
pub fn parse_input(_raw: &str) -> Result<(), ()> { Err(()) }
`,
    'tests/parser.rs': `
use fixture::parser::parse_input;

#[test]
fn malformed_input_is_rejected() {
    let result = parse_input("bad");
    match result {
        Err(error) => assert_eq!(error, ()),
        Ok(()) => panic!("malformed input must fail"),
    }
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /parser parse_input lacks invalid-input test evidence/u, output);
  assert.doesNotMatch(output, /parser parse_input lacks invalid\/empty\/oversized\/malformed test evidence/u, output);
});

test('parser rejection evidence rejects a match that accepts the error path without observation', () => {
  const project = makeProject({
    'src/parser.rs': `
pub fn parse_input(_raw: &str) -> Result<(), ()> { Err(()) }
`,
    'tests/parser.rs': `
use fixture::parser::parse_input;

#[test]
fn malformed_input_is_not_really_checked() {
    let result = parse_input("bad");
    match result {
        Err(_) => {},
        Ok(value) => assert_eq!(value, ()),
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /parser parse_input lacks invalid-input test evidence/u, output);
});

test('nonfallible parsers may prove invalid input through an asserted empty result', () => {
  const project = makeProject({
    'src/parser.rs': `
pub fn parse_ledger(_raw: &str) -> Vec<String> { Vec::new() }
`,
    'tests/parser.rs': `
use fixture::parser::parse_ledger;

#[test]
fn malformed_ledger_is_rejected_as_empty() {
    let rows = parse_ledger("not a ledger");
    assert!(rows.is_empty());
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /parser parse_ledger lacks invalid-input test evidence/u, output);
  assert.doesNotMatch(output, /parser parse_ledger lacks invalid\/empty\/oversized\/malformed test evidence/u, output);
});

test('RR-14.23 does not require identity conversions for standalone boundary records', () => {
  const project = makeProject({
    'src/boundary/presentation.rs': `
use serde::{Deserialize, Serialize};

// ROUNDTRIP-TEST: the outbound response wire shape is round-tripped for binding drift.
#[derive(Serialize, Deserialize)]
pub struct ReportResponse {
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuditEnvelope {
    pub value: String,
}

pub struct RouteRequest {
    pub value: String,
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.23/u, output);
});

test('RR-14.23 requires conversion only for DTOs with a separate domain counterpart', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct FirstDto {
    pub value: String,
}

pub struct SecondDto {
    pub value: String,
}

pub struct First {
    pub value: String,
}

pub struct Second {
    pub value: String,
}

pub struct StandaloneDto {
    pub value: String,
}

impl From<FirstDto> for First {
    fn from(input: FirstDto) -> Self {
        Self { value: input.value }
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO struct FirstDto lacks explicit domain conversion/u, output);
  assert.match(output, /DTO struct SecondDto lacks explicit domain conversion/u, output);
  assert.doesNotMatch(output, /DTO struct StandaloneDto lacks explicit domain conversion/u, output);
});

test('RR-14.23 accepts per-DTO trait and named mapper conversions', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct FirstDto {
    pub value: String,
}

pub struct SecondRequest {
    pub value: String,
}

pub struct ThirdDto {
    pub value: String,
}

pub struct First {
    pub value: String,
}

pub struct Second {
    pub value: String,
}

pub struct Third {
    pub value: String,
}

impl TryFrom<FirstDto> for First {
    type Error = &'static str;
    fn try_from(input: FirstDto) -> Result<Self, Self::Error> {
        Ok(Self { value: input.value })
    }
}

fn map_to_domain(input: SecondRequest) -> Second {
    Second { value: input.value }
}

impl From<Third> for ThirdDto {
    fn from(input: Third) -> Self {
        Self { value: input.value }
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.23/u, output);
});
