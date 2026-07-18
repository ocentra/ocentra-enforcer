import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures, rr1227Fixture, rr1227Output } from './rust-rules-domain-evidence-fixture.mjs';

test('RR-4.20 and RR-4.21 accept manual Error implementations that expose sources', () => {
  const project = makeProject({
    'src/lib.rs': `
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("app error")
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Io(error) => Some(error) }
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-4\.(?:20|21)/u, output);
});

test('RR-4.21 still rejects manual Error wrappers that hide their source', () => {
  const project = makeProject({
    'src/lib.rs': `
#[derive(Debug)]
pub enum AppError { Io(std::io::Error) }
impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { formatter.write_str("app error") }
}
impl std::error::Error for AppError {}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /RR-4\.21/u, output);
});

test('RR-6.50 recognizes generic manual Debug implementations', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct EventRecorder<E> { values: Vec<E> }
impl<E> std::fmt::Debug for EventRecorder<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("EventRecorder").finish_non_exhaustive()
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-6\.50/u, output);
});

test('RR-12.25 treats named assertion helpers as behavioral evidence', () => {
  const project = makeProject({
    'crates/fixture/tests/ids.rs': `
#[test]
fn rejects_invalid_id() {
    assert_invalid(UserId::parse(" "));
}
fn assert_invalid<T>(_result: T) {}
struct UserId;
impl UserId { fn parse(_value: &str) -> Result<Self, ()> { Err(()) } }
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.25/u, output);
});

test('RR-12.25 treats the fixture parity harness as behavioral evidence without accepting lookalikes', () => {
  const parityProject = makeProject({
    'crates/fixture/tests/parity.rs': `
#[test]
fn validator_holds_fixture_parity() {
    run_fixture_parity(&Validator::new(), "fail.rs", "pass.rs");
}
struct Validator;
impl Validator { fn new() -> Self { Self } }
fn run_fixture_parity<T>(_validator: &T, _fail: &str, _pass: &str) {}
`,
  });
  const parityResult = runGate(parityProject);
  const parityOutput = `${parityResult.stdout}\n${parityResult.stderr}`;
  assert.doesNotMatch(parityOutput, /RR-12\.25/u, parityOutput);

  const lookalikeProject = makeProject({
    'crates/fixture/tests/parity.rs': `
#[test]
fn validator_only_builds_a_fixture() {
    run_fixture_builder(Validator::new());
}
struct Validator;
impl Validator { fn new() -> Self { Self } }
fn run_fixture_builder<T>(_validator: T) {}
`,
  });
  const lookalikeResult = runGate(lookalikeProject);
  const lookalikeOutput = `${lookalikeResult.stdout}\n${lookalikeResult.stderr}`;
  assert.match(lookalikeOutput, /RR-12\.25/u, lookalikeOutput);
});

test('test-structure rules balance real bodies instead of matching fixture strings', () => {
  const project = makeProject({
    'crates/fixture/tests/bodies.rs': String.raw`
#[test]
fn nonempty_result_test_with_embedded_source() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = "fn embedded_fixture() {}";
    assert_eq!(fixture, "fn embedded_fixture() {}");
    Ok(())
}

#[test]
fn nested_construction_with_behavior() {
    let parsed = {
        UserId::parse("user-1")
    };
    assert_invalid(parsed);
}

#[test]
fn genuinely_empty() {}

#[test]
fn genuinely_construction_only() {
    let _parsed = UserId::parse("user-1");
}

fn assert_invalid<T>(_result: T) {}
struct UserId;
impl UserId { fn parse(_value: &str) -> Result<Self, ()> { Err(()) } }
`,
  });
  const result = runGateArgs(project, ['scan', '--json']);
  assert.notEqual(result.stdout.trim(), '', result.stderr);
  const report = JSON.parse(result.stdout);
  const testStructureFindings = report.violations.filter(
    (violation) => ['RR-12.24', 'RR-12.25'].includes(violation.ruleId),
  );
  assert.deepEqual(
    testStructureFindings.map((violation) => violation.ruleId),
    ['RR-12.24', 'RR-12.25'],
    result.stdout,
  );
});

test('transport suffixes require boundary placement only when they are DTOs or serde records', () => {
  const project = makeProject({
    'src/domain.rs': `
pub struct EventEnvelope<E> { pub payload: E }

#[derive(serde::Serialize)]
pub struct WireEnvelope { pub payload: String }
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /DTO struct EventEnvelope/u, output);
  assert.match(output, /DTO struct WireEnvelope is outside/u, output);
});

test('async runtime fire-and-forget and unbounded channels fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
use std::sync::{Arc, Mutex};

pub async fn run_worker() {
    let state = std::sync::Mutex::new(1_u64);
    let _ = std::fs::read("state.txt");
    tokio::spawn(async move {});
    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    client.send().await;
    loop {
        process().await;
    }
}

pub fn runtime() {
    let _runtime = tokio::runtime::Runtime::new();
    futures::executor::block_on(async {});
}

pub fn share(state: Arc<Mutex<u64>>) {
    let _ = state;
}
`,
    'src/timing_test.rs': `
#[test]
fn waits() {
    std::thread::sleep(std::time::Duration::from_millis(1));
}
`,
  });
  expectFailures(project, [
    'RR-8.16',
    'RR-8.18',
    'RR-8.19',
    'RR-8.20',
    'RR-8.21',
    'RR-8.23',
    'RR-8.25',
    'RR-8.27',
    'RR-8.28',
    'RR-8.29',
    'RR-8.30',
  ]);
});

test('finite async reads and bounded test routing do not require cancellation evidence', () => {
  const project = makeProject({
    'src/read.rs': `
pub async fn read_once() -> Result<(), ()> {
    Ok(())
}
`,
    'tests/routing.rs': `
#[tokio::test]
async fn routes_a_typed_variant_through_a_bounded_channel() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<u8>(1);
    sender.send(7).await.expect("receiver remains available during this finite test");
    assert_eq!(receiver.recv().await, Some(7));
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.29/u, output);
});

test('RR-8.21 does not classify bounded timer sleep with a delay getter as external I/O', () => {
  const project = makeProject({
    'src/retry.rs': `
pub async fn retry_after(delay: RetryDelay) {
    tokio::time::sleep(delay.get()).await;
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-8\.21/u, output);
});

test('RR-12.27 accepts same-crate external property tests that reference the target parser', () => {
  const project = rr1227Fixture(`
use fixture::{parse_widget, ParseInput};
use proptest::prelude::*;

proptest! {
    #[test]
    fn parser_is_total(_seed in any::<u8>()) {
        let _output = parse_widget(ParseInput);
    }
}
`);
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /RR-12\.27/u, output);
});

test('RR-12.27 rejects ordinary unit examples as property evidence', () => {
  const project = rr1227Fixture(`
use fixture::{parse_widget, ParseInput};

#[test]
fn parses_one_example() {
    let _output = parse_widget(ParseInput);
}
`);
  assert.match(rr1227Output(project), /RR-12\.27/u);
});

test('RR-12.27 rejects unrelated property tests in the same crate', () => {
  const project = rr1227Fixture(`
use proptest::prelude::*;

proptest! {
    #[test]
    fn unrelated_arithmetic_is_total(value in any::<u8>()) {
        prop_assert_eq!(value.wrapping_add(0), value);
    }
}
`);
  assert.match(rr1227Output(project), /RR-12\.27/u);
});

test('RR-12.27 still fails when property-test evidence is missing', () => {
  const project = rr1227Fixture(null);
  assert.match(rr1227Output(project), /RR-12\.27/u);
});

test('RR-12.27 registered property evidence is exact per source path and function', () => {
  const project = makeProject({
    'src/alpha.rs': `
pub fn parse(input: &str) -> &str { input }
`,
    'src/beta.rs': `
pub fn parse(input: &str) -> &str { input }
`,
    'tests/property.rs': `
use proptest::prelude::*;

macro_rules! property_parser_contracts {
    ($($key:literal => $target:path),+ $(,)?) => {
        proptest! {
            #[test]
            fn registered_parsers_are_total(input in ".{0,32}") {
                $(let _ = $target(&input);)+
            }
        }
    };
}

property_parser_contracts! {
    "src/alpha.rs::parse" => fixture::alpha::parse,
}
`,
  });
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /src[\\/]alpha\.rs[^\n]*RR-12\.27/u, output);
  assert.match(output, /src[\\/]beta\.rs[^\n]*RR-12\.27/u, output);
});

test('Rust documentation survives intervening derive and serde attributes', () => {
  const project = makeProject({
    'src/lib.rs': `
/// Wire response returned by the parser.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResponse {
    pub value: String,
}
`,
  });
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /DOC-1\.1/u, output);
});

test('serde defaults accept nearby semantic rationale without a policy marker', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
#[derive(serde::Deserialize)]
pub struct InputDto {
    /// Omitted values represent an empty upstream result set.
    #[serde(default)]
    pub values: Vec<String>,
}
`,
  });
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /RR-14\.19/u, output);
});

test('infallible From DTO conversions do not demand negative rejection tests', () => {
  const project = makeProject({
    'src/boundary/input.rs': `
pub struct InputDto {
    value: String,
}
pub struct Input {
    value: String,
}
impl From<InputDto> for Input {
    fn from(input: InputDto) -> Self {
        Self { value: input.value }
    }
}
`,
  });
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /RR-12\.18/u, output);
});

test('proptest assertions count as behavioral construction assertions', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct Widget;',
    'tests/property.rs': `
use proptest::{prelude::any, prop_assert_eq, proptest};
use fixture::Widget;
proptest! {
    #[test]
    fn constructs_and_observes(value in any::<u8>()) {
        let _widget = Widget::new();
        prop_assert_eq!(value, value);
    }
}
`,
  });
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /RR-12\.25/u, output);
});

test('parser prose mentioning an engine binary is not classified as a binary parser', () => {
  const project = rr1227Fixture(`
use fixture::{parse_widget, ParseInput};
use proptest::prelude::*;

proptest! {
    #[test]
    fn parser_is_total(_seed in any::<u8>()) {
        let _output = parse_widget(ParseInput);
    }
}
`);
  fs.writeFileSync(
    path.join(project, 'src/lib.rs'),
    fs.readFileSync(path.join(project, 'src/lib.rs'), 'utf8').replace(
      '/// Parses validated input;',
      '/// Parses recorded output from an optional engine binary;',
    ),
  );
  const output = rr1227Output(project);
  assert.doesNotMatch(output, /RR-12\.28/u, output);
});

test('background tasks still require cancellation evidence', () => {
  const project = makeProject({
    'src/worker.rs': `
pub async fn start_background_work() {
    tokio::spawn(async move {});
}
`,
  });
  expectFailure(project, 'RR-12.29');
});

test('async loops still require cancellation evidence', () => {
  const project = makeProject({
    'src/worker.rs': `
pub async fn drain_forever() {
    loop {
        tokio::task::yield_now().await;
    }
}
`,
  });
  expectFailure(project, 'RR-12.29');
});

test('Rust serde domain derives and weak assertions fail scanner', () => {
  const rustAssertMacro = 'assert';
  const project = makeProject({
    'src/lib.rs': `
#[derive(Deserialize)]
#[serde(untagged)]
pub enum UserEnvelope {
    Named { name: String },
}

pub fn verify(result: Result<u64, ()>, value: Option<u64>) {
    ${rustAssertMacro}!(result.is_ok());
    ${rustAssertMacro}!(value.is_some());
}
`,
  });
  expectFailures(project, ['RR-12.22', 'RR-12.23', 'RR-14.16', 'RR-14.18']);
});
