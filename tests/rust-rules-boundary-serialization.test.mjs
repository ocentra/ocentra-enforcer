import test from 'node:test';
import assert from 'node:assert/strict';
import { STRING_ERROR_RULE_IDS } from './rust-rules-rule-groups.mjs';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-fixture.mjs';
test('runtime inline string guard can be enabled from config', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      enforceRuntimeStringLiterals: true,
      rawTypeBoundaryGlobs: ['src/lib.rs'],
    }),
    'src/lib.rs': `
pub fn route_name() -> &'static str { "devices" }
`,
  });
  expectFailure(project, 'RR-18.16');
});

test('serialized raw identity fields fail when Ocentra-style guard is enabled', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      enforceSerializedPublicDomainPrimitives: true,
      rawTypeBoundaryGlobs: ['src/lib.rs'],
    }),
    'src/lib.rs': `
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub event_id: String,
}
`,
  });
  expectFailure(project, 'RR-6.26');
});

test('structural transport records are boundaries without masking domain lookalikes', () => {
  const project = makeProject({
    'src/lib.rs': `
#[derive(Serialize, Deserialize)]
pub struct AccountWire {
    pub account_id: String,
    pub note: Option<String>,
}

pub struct LookupRequest {
    pub account_id: String,
}

pub struct DomainWire {
    pub account_id: String,
}

pub struct RequestState {
    pub account_id: String,
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json']);
  const failures = JSON.parse(result.stdout).findings;
  const rawFieldLines = failures
    .filter((failure) => failure.ruleId === 'RR-6.3')
    .map((failure) => failure.line);
  assert.deepEqual(rawFieldLines, [12, 16]);
  assert.equal(
    failures.some((failure) => failure.ruleId === 'RR-14.16' && failure.line === 2),
    false,
  );
  assert.equal(
    failures.some((failure) => failure.ruleId === 'RR-14.17' && failure.line === 2),
    false,
  );
});

test('stringly Rust errors and swallowed results fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;

pub fn parse_user(raw: &str) -> Result<UserId, String> {
    let _ = raw.parse::<u64>();
    raw.parse::<u64>()
        .map_err(|e| e.to_string())
        .ok()
        .unwrap_or_default();
    Err("bad user")
}

pub fn parse_group(raw: &str) -> Result<UserId, &'static str> {
    Err(format!("bad {raw}"))
}

pub fn save_user(raw: &str) -> bool {
    raw.parse::<u64>().is_ok()
}

pub fn find_user(raw: &str) -> i32 {
    if raw.is_empty() {
        return -1;
    }
    1
}

pub struct ParsedUserId(u64);

impl ParsedUserId {
    pub fn new(raw: String) -> Self {
        Self(raw.parse::<u64>().unwrap_or(0))
    }
}

pub enum ParseError {
    Io(std::io::Error),
}

pub fn read_user(raw: &str) -> Result<UserId, ParseError> {
    let value = raw.parse::<u64>().unwrap_or(0);
    error!("failed {value}");
    Err(ParseError::Io(std::io::Error::other("bad")))
}

fn main() {
    let _ = run();
}

fn run() -> Result<(), ParseError> {
    Ok(())
}
`,
  });
  expectFailures(project, STRING_ERROR_RULE_IDS);
});
