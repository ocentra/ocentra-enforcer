import test from 'node:test';
import assert from 'node:assert/strict';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-domain-evidence-fixture.mjs';

test('dangerous Rust source primitives fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId(u64);
static mut READY: bool = false;
use core::cell::UnsafeCell;
use core::mem::{ManuallyDrop, MaybeUninit};

extern "C" {
    fn foreign();
}

#[no_mangle]
pub extern "C" fn exported() {}

pub fn cast_user(raw: u64) -> UserId {
    let ptr = &raw as *const u64;
    let _slot: MaybeUninit<u64> = MaybeUninit::uninit();
    let _manual = ManuallyDrop::new(raw);
    let _cell = UnsafeCell::new(raw);
    let _leaked = Box::leak(Box::new(raw));
    core::mem::forget(_manual);
    let _unchecked = [raw].get_unchecked(0);
    unsafe { core::mem::transmute::<u64, UserId>(*ptr) }
}

unsafe impl Send for UserId {}
`,
    'src/ffi/api.rs': `
pub struct RawFfi {
    pub value: u64,
}
`,
    'src/unsafe_escape_test.rs': `
#[allow(unsafe_code)]
pub fn test_escape(ptr: *const u64) -> u64 {
    unsafe { *ptr }
}
`,
  });
  expectFailures(project, [
    'RR-3.16',
    'RR-3.17',
    'RR-3.18',
    'RR-3.19',
    'RR-3.20',
    'RR-3.21',
    'RR-3.22',
    'RR-3.23',
    'RR-3.24',
    'RR-3.25',
    'RR-3.26',
    'RR-3.27',
    'RR-3.28',
    'RR-3.32',
    'RR-3.33',
  ]);
});

test('domain generic string escape hatches and bool state clusters fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::borrow::Cow;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct UserId;
pub struct RawUserId(pub String);
pub struct NumericUserId(u64);
pub struct User;
pub enum LookupError { Missing }
type UserIds = Vec<UserId>;

pub struct WorkflowState {
    active: bool,
    pending: bool,
    failed: bool,
    enabled: Option<bool>,
    name: Option<String>,
    timeout: Duration,
    created_at: SystemTime,
    url: String,
    file_path: String,
    user_id: String,
    data: serde_json::Value,
    by_name: BTreeMap<String, User>,
}

#[derive(Debug)]
pub struct ApiToken(String);

impl RawUserId {
    pub fn new(width: u32, height: u32) -> Self {
        Self(width.checked_add(height).unwrap().to_string())
    }
}

pub fn load_user<T: AsRef<str>>(id: T) -> Result<User, LookupError> {
    let _ = id;
    Err(LookupError::Missing)
}

pub fn show_user(id: impl Display) -> Result<User, LookupError> {
    let _ = id;
    Err(LookupError::Missing)
}

pub fn borrow_user(id: Cow<'_, str>) -> Result<User, LookupError> {
    let _ = id;
    Err(LookupError::Missing)
}

pub fn rename_user<T: Into<String>>(name: T) -> Result<User, LookupError> {
    let _ = name;
    Err(LookupError::Missing)
}

pub fn load_many(ids: Vec<String>) -> Result<Vec<User>, LookupError> {
    let _ = ids;
    Err(LookupError::Missing)
}

pub fn load_map(ids: HashMap<String, UserId>) -> Result<Vec<User>, LookupError> {
    let _ = ids;
    Err(LookupError::Missing)
}

pub fn load_sorted(map: BTreeMap<String, User>) -> Result<Vec<User>, LookupError> {
    let _ = map;
    Err(LookupError::Missing)
}

pub fn load_url(url: String, file_path: String, timeout: Duration) -> (UserId, User) {
    let _ = (url, file_path, timeout);
    (UserId, User)
}

pub fn share_state(state: Arc<Mutex<User>>) {
    let _ = state;
}
`,
  });
  expectFailures(project, [
    'RR-6.27',
    'RR-6.28',
    'RR-6.29',
    'RR-6.30',
    'RR-6.31',
    'RR-6.32',
    'RR-6.33',
    'RR-6.34',
    'RR-6.35',
    'RR-6.36',
    'RR-6.37',
    'RR-6.38',
    'RR-6.39',
    'RR-6.40',
    'RR-6.41',
    'RR-6.42',
    'RR-6.43',
    'RR-6.45',
    'RR-6.46',
    'RR-6.47',
    'RR-6.48',
    'RR-6.49',
    'RR-6.51',
    'RR-8.30',
  ]);
});

test('validator class names are not treated as secret value types', () => {
  const project = makeProject({
    'src/lib.rs': `
#![forbid(unsafe_code)]

pub struct RuleId;

/// Detects plaintext password handling without storing a password.
#[derive(Debug)]
pub struct PlaintextPasswordValidator {
    rule_id: RuleId,
}

/// Detects insecure token generation without storing a token.
#[derive(Debug)]
pub struct InsecureRandomTokenValidator {
    rule_id: RuleId,
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-6\.51/u);
  assert.doesNotMatch(output, /RR-6\.52/u);
});

test('RR-6.50 classifies value objects but not validator services or registry rows', () => {
  const project = makeProject({
    'src/lib.rs': `
#![forbid(unsafe_code)]

pub struct RuleId;

/// Domain identity value object intentionally missing Debug.
pub struct UserId(u64);

/// Operational validator service, not a domain value object.
pub struct UserPolicyValidator {
    rule_id: RuleId,
}

/// Registry transport row, not a domain value object.
pub struct RegistryRow {
    rule_id: RuleId,
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /public domain value object UserId lacks intentional Debug/u, output);
  assert.doesNotMatch(output, /public domain value object UserPolicyValidator/u, output);
  assert.doesNotMatch(output, /public domain value object RegistryRow/u, output);
});

test('RR-14.25 does not require round-trip evidence for outbound Serialize-only DTOs', () => {
  const project = makeProject({
    'src/boundary/presentation.rs': `
use serde::Serialize;

#[derive(Serialize)]
pub struct ReportDto {
    pub value: String,
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.25/u, output);
});

test('RR-14.25 still requires evidence for inbound and bidirectional DTOs', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct InputDto {
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct RecordDto {
    pub value: String,
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
  assert.match(output, /DTO struct RecordDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts explicit round-trip evidence for deserialized DTOs', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
// BOUNDARY-INVARIANT: this module owns the external input wire shape.
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
    let value = InputDto { value: String::new() };
    let encoded = serde_json::to_vec(&value).unwrap();
    let decoded: InputDto = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.value, value.value);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.25/u, output);
});

test('RR-14.25 accepts an inline typed codec decode compared with its encoded DTO', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;

#[test]
fn input_dto_round_trips_inline() {
    let original = InputDto { value: String::new() };
    let wire = serde_json::to_string(&original).unwrap();
    assert_eq!(serde_json::from_str::<InputDto>(&wire).unwrap(), original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts a nested codec round trip only when the decoder consumes that DTO encoding', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;

#[test]
fn input_dto_round_trips_through_nested_codec_calls() {
    let original = InputDto { value: String::from("value") };
    let decoded: InputDto =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    assert_eq!(decoded, original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects nested codec calls when the decoder consumes another value', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::InputDto;

#[test]
fn unrelated_nested_codec_calls_are_not_input_evidence() {
    let original = InputDto { value: String::from("value") };
    let unrelated = String::from("unrelated");
    let _original_wire = serde_json::to_vec(&original).unwrap();
    let decoded: InputDto =
        serde_json::from_slice(&serde_json::to_vec(&unrelated).unwrap()).unwrap();
    assert_eq!(decoded, original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts a named decoder only when its body invokes a real codec for the DTO', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }

pub fn decode_input_json(payload: &str) -> Result<InputDto, serde_json::Error> {
    serde_json::from_str(payload)
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::{decode_input_json, InputDto};

#[test]
fn input_dto_round_trips_through_its_decoder() {
    let original = InputDto { value: String::new() };
    let wire = serde_json::to_string(&original).unwrap();
    assert_eq!(decode_input_json(&wire).unwrap(), original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 rejects a named decoder lookalike that never invokes a codec', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct InputDto { pub value: String }

pub fn decode_input_json(_payload: &str) -> InputDto {
    InputDto { value: String::new() }
}
`,
    'tests/input_roundtrip.rs': `
use fixture::boundary::input::{decode_input_json, InputDto};

#[test]
fn decoder_lookalike_is_not_round_trip_evidence() {
    let original = InputDto { value: String::new() };
    let wire = serde_json::to_string(&original).unwrap();
    assert_eq!(decode_input_json(&wire), original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /DTO struct InputDto lacks round-trip test evidence/u, output);
});

test('RR-14.25 accepts a mechanically validated generic helper with an exact DTO type argument', () => {
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
    assert_round_trip::<InputDto>(&InputDto { value: String::new() });
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.25/u, output);
});

test('RR-14.25 accepts a validated generic return helper only when the caller compares the same DTO', () => {
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

fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
where
    T: Serialize + DeserializeOwned,
{
    serde_json::from_slice(&serde_json::to_vec(value)?)
}

#[test]
fn input_dto_round_trip_accepts_the_wire_shape() {
    let original = InputDto { value: String::new() };
    assert_eq!(round_trip(&original).unwrap(), original);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-14\.25/u, output);
});
