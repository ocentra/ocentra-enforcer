import test from 'node:test';
import assert from 'node:assert/strict';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-fixture.mjs';
test('good branded-domain fixture passes scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
#![forbid(unsafe_code)]
#![deny(warnings)]

use core::num::NonZeroU64;

/// User identifier.
/// BRAND-INVARIANT: the inner value is non-zero and issued by the identity service.
#[derive(Debug)]
pub struct UserId(NonZeroU64);

/// User record.
#[derive(Debug)]
pub struct UserRecord {
    id: UserId,
}

/// Lookup failure.
#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    /// The user does not exist.
    NotFound,
}

/// Finds a user by branded identifier.
pub fn find_user(id: UserId) -> Result<Option<UserRecord>, LookupError> {
    let _ = id;
    Ok(None)
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('RR-14.25 accepts an external test body naming the DTO with round-trip semantics', () => {
  const project = makeProject({
    'src/boundary/model.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalModelDto {
    pub value: String,
}
`,
    'tests/model_roundtrip.rs': `
use fixture::boundary::model::ExternalModelDto;

#[test]
fn external_model_dto_round_trip_preserves_the_wire_shape() {
    let value = ExternalModelDto { value: "ok".to_owned() };
    let encoded = serde_json::to_vec(&value).unwrap();
    let decoded: ExternalModelDto = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, value);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.25/u, output);
});

test('RR-14.25 rejects a DTO test that lacks round-trip semantics', () => {
  const project = makeProject({
    'src/boundary/model.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalModelDto {
    pub value: String,
}
`,
    'tests/model_contract.rs': `
use fixture::boundary::model::ExternalModelDto;

#[test]
fn external_model_dto_constructs() {
    let value = ExternalModelDto { value: "ok".to_owned() };
    assert_eq!(value.value, "ok");
}
`,
  });
  expectFailure(project, 'RR-14.25');
});

test('RR-14.25 rejects comment-only round-trip claims', () => {
  const project = makeProject({
    'src/boundary/model.rs': `
use serde::{Deserialize, Serialize};

// ROUNDTRIP-TEST: claimed_without_executable_test_evidence
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalModelDto {
    pub value: String,
}
`,
  });
  expectFailure(project, 'RR-14.25');
});

test('presentation-boundary modules are classified as raw-type boundaries', () => {
  const project = makeProject({
    'src/topology_presentation_boundary.rs': `
/// Formats a topology report at the outbound presentation boundary.
pub fn rendered_text(value: &str) -> &str { value }
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('RR-6.4 only classifies record fields, not multiline function parameters', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_cli(
    args: &[String],
    index: &mut usize,
) -> bool {
    !args.is_empty() && *index == 0
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-6\.4/u, output);
});

test('private literal-analyzer helpers may use raw lexer cursor inputs', () => {
  const project = makeProject({
    'crates/enforcer-literal-scan/src/lexer.rs': `
pub(crate) fn scan_literal(source: &str, index: usize, output: &mut Vec<String>) -> bool {
    output.push(source.to_owned());
    index < source.len()
}

pub(crate) struct LexerCursor {
    pub(crate) source: String,
    pub(crate) index: usize,
    pub(crate) file_path: String,
    pub(crate) context: Option<String>,
}

fn build_cursor() -> LexerCursor {
    LexerCursor {
        source: String::new(),
        index: 0,
        file_path: String::new(),
        context: None,
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-6\.[12]/u, output);
  assert.doesNotMatch(output, /RR-6\.(?:3|4|35|41|42)/u, output);
});

test('literal analyzers may default absent optional lexer context', () => {
  const project = makeProject({
    'crates/enforcer-literal-scan/src/lexer_shared.rs': `
fn context_or_empty(context: Option<String>) -> String {
    context.unwrap_or_default()
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-4\.18/u, output);
});
