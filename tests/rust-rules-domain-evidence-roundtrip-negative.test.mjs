import test from 'node:test';
import assert from 'node:assert/strict';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-domain-evidence-fixture.mjs';

test('RR-14.25 rejects a generic helper that serializes an unrelated value', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto {
    pub value: String,
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(_value: &T)
where
    T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let unrelated = String::new();
    let encoded = serde_json::to_vec(&unrelated).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(core::mem::size_of_val(&decoded), core::mem::size_of::<T>());
}

#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {
    assert_round_trip::<InputDto>(&InputDto { value: String::new() });
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects a valid helper call associated only with an unrelated value', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto {
    pub value: String,
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let encoded = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {
    let _target = core::mem::size_of::<InputDto>();
    let unrelated = String::new();
    assert_round_trip(&unrelated);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts an inferred generic helper for an inline DTO literal', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let encoded = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn inline_input_dto_round_trips() {
    assert_round_trip(&InputDto { value: String::new() });
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts an inferred generic helper for a unit DTO value', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto;
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let encoded = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn unit_input_dto_round_trips() {
    let input = InputDto;
    assert_round_trip(&input);
    assert_round_trip(&InputDto);
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects an inferred local produced by an unrelated associated function', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto;

impl InputDto {
    pub fn unrelated_json() -> serde_json::Value { serde_json::json!({}) }
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let encoded = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn unrelated_factory_local_is_not_dto_evidence() {
    let unrelated = InputDto::unrelated_json();
    assert_round_trip(&unrelated);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts an inferred factory call only when its signature returns the DTO', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }

impl InputDto {
    pub fn empty() -> Self { Self { value: String::new() } }
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let encoded = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn factory_input_dto_round_trips() { assert_round_trip(&InputDto::empty()); }
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects an inferred factory lookalike returning another type', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto { pub value: String }

impl InputDto {
    pub fn unrelated_json() -> serde_json::Value { serde_json::json!({ "value": "" }) }
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;
use serde::{de::DeserializeOwned, Serialize};

fn assert_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let encoded = serde_json::to_vec(value).unwrap();
    let decoded: T = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn unrelated_factory_is_not_dto_evidence() {
    assert_round_trip(&InputDto::unrelated_json());
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts nested DTO evidence only through a typed decoded projection', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct NestedDto { pub value: String }

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct MiddleResponse { pub nested: NestedDto }

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ParentResponse { pub middle: MiddleResponse }
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::{MiddleResponse, NestedDto, ParentResponse};

#[test]
fn parent_and_nested_dto_round_trip() {
    let original = ParentResponse {
        middle: MiddleResponse { nested: NestedDto { value: String::new() } },
    };
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded: ParentResponse = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, original);
    let middle: &MiddleResponse = &decoded.middle;
    let _: &NestedDto = &middle.nested;
}
`,
  });
  const output = `${runGate(project).stdout}`;
  assert.doesNotMatch(output, /DTO struct NestedDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects a parent round trip with only an unrelated nested DTO mention', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct NestedDto { pub value: String }

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ParentResponse { pub value: String }
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::{NestedDto, ParentResponse};

#[test]
fn parent_round_trip_does_not_cover_unrelated_nested_dto() {
    let original = ParentResponse { value: String::new() };
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded: ParentResponse = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, original);
    let _mention = core::mem::size_of::<NestedDto>();
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct NestedDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts a mechanically validated persistence write-read cycle and nested projection', () => {
  const project = makeProject({
    'src/boundary/snapshot.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ChildDto { pub value: String }

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SnapshotResponse { pub child: ChildDto }
`,
    'src/store.rs': `
use crate::boundary::snapshot::{ChildDto, SnapshotResponse};

#[derive(Debug, PartialEq)]
pub struct Snapshot { pub value: String }

pub fn write_snapshot(path: &std::path::Path, snapshot: &Snapshot) -> std::io::Result<()> {
    let record = SnapshotResponse { child: ChildDto { value: snapshot.value.clone() } };
    let payload = serde_json::to_vec(&record).unwrap();
    std::fs::write(path, payload)
}

pub fn load_snapshot(path: &std::path::Path) -> std::io::Result<Snapshot> {
    let payload = std::fs::read(path)?;
    let record: SnapshotResponse = serde_json::from_slice(&payload).unwrap();
    Ok(Snapshot { value: record.child.value })
}
`,
    'tests/snapshot.rs': `
use fixture::boundary::snapshot::{ChildDto, SnapshotResponse};
use fixture::store::{load_snapshot, write_snapshot, Snapshot};

#[test]
fn snapshot_persistence_round_trips() {
    let path = std::path::PathBuf::from("snapshot.json");
    let original = Snapshot { value: String::from("value") };
    write_snapshot(&path, &original).unwrap();
    let loaded = load_snapshot(&path).unwrap();
    assert_eq!(loaded, original);
    let raw = std::fs::read_to_string(&path).unwrap();
    let decoded: SnapshotResponse = serde_json::from_str(&raw).unwrap();
    let child: &ChildDto = &decoded.child;
    assert_eq!(child.value, "value");
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO struct SnapshotResponse lacks round-trip test evidence/u, output);
  assert.doesNotMatch(output, /DTO struct ChildDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects a persistence lookalike whose writer encodes unrelated data', () => {
  const project = makeProject({
    'src/boundary/snapshot.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SnapshotResponse { pub value: String }
`,
    'src/store.rs': `
use crate::boundary::snapshot::SnapshotResponse;

#[derive(Debug, PartialEq)]
pub struct Snapshot { pub value: String }

pub fn write_snapshot(path: &std::path::Path, snapshot: &Snapshot) -> std::io::Result<()> {
    let _record = SnapshotResponse { value: snapshot.value.clone() };
    let unrelated = String::new();
    let payload = serde_json::to_vec(&unrelated).unwrap();
    std::fs::write(path, payload)
}

pub fn load_snapshot(path: &std::path::Path) -> std::io::Result<Snapshot> {
    let payload = std::fs::read(path)?;
    let record: SnapshotResponse = serde_json::from_slice(&payload).unwrap();
    Ok(Snapshot { value: record.value })
}
`,
    'tests/snapshot.rs': `
use fixture::store::{load_snapshot, write_snapshot, Snapshot};

#[test]
fn snapshot_persistence_lookalike() {
    let path = std::path::PathBuf::from("snapshot.json");
    let original = Snapshot { value: String::from("value") };
    write_snapshot(&path, &original).unwrap();
    let loaded = load_snapshot(&path).unwrap();
    assert_eq!(loaded, original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct SnapshotResponse lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects keyword-named construction-only evidence', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto {
    pub value: String,
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;

#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {
    let decoded = InputDto { value: String::new() };
    assert!(decoded.value.is_empty());
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects an empty keyword-named test', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto {
    pub value: String,
}
`,
    'tests/input_roundtrip.rs': `
#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects an unrelated codec round trip and assertion', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::Deserialize;

#[derive(Deserialize)]
pub struct InputDto {
    pub value: String,
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;

#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {
    let _dto_size = core::mem::size_of::<InputDto>();
    let value = String::new();
    let encoded = serde_json::to_vec(&value).unwrap();
    let decoded: String = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, value);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});
