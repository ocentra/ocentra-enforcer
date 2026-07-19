import test from 'node:test';
import assert from 'node:assert/strict';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-fixture.mjs';
test('unwrap fails with RR-4.1 and helpful output', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: UserId) -> Option<UserId> {
    Some(id).unwrap()
}
`,
  });
  expectFailure(project, 'RR-4.1');
});

test('raw string parameter fails with RR-6.1', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: &str) -> Option<UserId> {
    let _ = id;
    None
}
`,
  });
  expectFailure(project, 'RR-6.1');
});

test('raw primitive parameter fails with RR-6.2', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: u64) -> Option<UserId> {
    let _ = id;
    None
}
`,
  });
  expectFailure(project, 'RR-6.2');
});

test('RR-6.1, RR-6.2, and RR-6.4 distinguish private implementation state from public domain boundaries', () => {
  const project = makeProject({
    'src/lib.rs': `
struct ParserCursor {
    source: String,
    offset: usize,
}

fn normalize_cursor(source: &str, offset: usize) -> bool {
    !source.is_empty() && offset < source.len()
}

pub struct UserIdentity {
    value: String,
    generation: usize,
}

pub fn load_identity(value: &str, generation: usize) -> Option<UserIdentity> {
    let _ = (value, generation);
    None
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json']);
  const report = JSON.parse(result.stdout);
  const rr6 = report.findings.filter((finding) =>
    ['RR-6.1', 'RR-6.2', 'RR-6.4'].includes(finding.ruleId),
  );
  assert.deepEqual(
    rr6.map((finding) => [finding.ruleId, finding.line]),
    [
      ['RR-6.4', 11],
      ['RR-6.4', 12],
      ['RR-6.1', 15],
      ['RR-6.2', 15],
    ],
    result.stdout,
  );
});

test('private language-analyzer AST predicates and counters are not domain APIs', () => {
  const project = makeProject({
    'crates/enforcer-lang-rust/src/rules/parser.rs': `
struct Visitor {
    depth: usize,
}

fn is_ast_match(node: &syn::Expr) -> bool {
    let _ = node;
    true
}

fn source_line(node: &syn::Expr) -> u32 {
    let _ = node;
    1
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});
test('private language-analyzer lexical string helpers are not domain APIs', () => {
  const project = makeProject({
    'crates/enforcer-lang-ts/src/rules/parser.rs': `
fn import_target(line: &str) -> Option<&str> {
    line.split_once(" from ").map(|(_, target)| target)
}

fn has_marker(source: &str) -> bool {
    source.contains("marker")
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('ordinary validator constructors that call branded parsers are not parser definitions', () => {
  const project = makeProject({
    'crates/enforcer-lang-ts/src/rules/validator.rs': `
use core::str::FromStr;

/// Branded rule identifier.
#[derive(Debug)]
pub struct RuleId;
impl FromStr for RuleId {
    type Err = core::convert::Infallible;
    fn from_str(_value: &str) -> Result<Self, Self::Err> { Ok(Self) }
}

/// A validator keyed by one branded rule identifier.
#[derive(Debug)]
pub struct Validator { rule_id: RuleId }
impl Validator {
    /// Builds the fixed-rule validator.
    pub fn new() -> Result<Self, core::convert::Infallible> {
        Ok(Self { rule_id: "TS-1.1".parse()? })
    }
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('derive attributes between rustdoc and public items preserve documentation evidence', () => {
  const project = makeProject({
    'src/lib.rs': `
/// Canonical validator state.
#[derive(Debug)]
pub struct ValidatorState;
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('unsafe wording inside rule-table strings remains masked from unsafe evidence rules', () => {
  const project = makeProject({
    'src/lib.rs': `
struct RuleSpec { title: &'static str }
const SPEC: RuleSpec = RuleSpec {
    title: "ESLint must enforce unsafe TypeScript rules",
};
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('unparseable source regression test satisfies parser negative evidence', () => {
  const project = makeProject({
    'crates/enforcer-lang-rust/src/rules/parser.rs': `
fn inspect_source() {
    let _ = syn::parse_file("fn valid() {}");
    drop("1".parse::<u32>());
}

#[cfg(test)]
mod tests {
    #[test]
    fn unparseable_source_stays_silent() {
        assert_eq!(
            syn::parse_file("not valid rust {{{")
                .err()
                .map(|error| error.to_string()),
            Some("cannot parse string into token stream".to_owned()),
        );
    }
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('parser evidence is associated with the parser exercised by direct edge-case tests', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_macro_args(input: &str) -> Result<Vec<&str>, &'static str> {
    if input == "bad" { return Err("invalid"); }
    Ok(input.split(',').collect())
}

#[cfg(test)]
mod tests {
    use super::parse_macro_args;

    #[test]
    fn parse_macro_args_rejects_malformed_tokens() {
        assert!(parse_macro_args("bad").is_err());
    }

    #[test]
    fn parse_macro_args_handles_empty_input() {
        assert!(parse_macro_args("").is_ok());
    }

    #[test]
    fn parse_macro_args_handles_oversized_argument_list() {
        assert!(parse_macro_args("a,b,c").is_ok());
    }

    #[test]
    fn parse_macro_args_rejects_invalid_non_expression_input() {
        assert!(parse_macro_args("bad").is_err());
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.(?:16|17)/u, output);
});

test('parser evidence for another function does not hide an untested parser', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_tested(input: &str) -> Result<&str, &'static str> {
    if input.is_empty() { Err("invalid") } else { Ok(input) }
}

fn parse_untested(input: &str) -> Result<&str, &'static str> {
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::parse_tested;

    #[test]
    fn parse_tested_rejects_invalid_input() {
        assert!(parse_tested("").is_err());
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /parser parse_untested lacks invalid-input test evidence/u, output);
  assert.match(output, /parser parse_untested lacks invalid\/empty\/oversized\/malformed test evidence/u, output);
  assert.doesNotMatch(output, /parser parse_tested lacks/u, output);
});

test('keyword-named parser tests require a target call and behavioral assertion', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_input(input: &str) -> Result<&str, &'static str> {
    if input.is_empty() { Err("invalid") } else { Ok(input) }
}

#[cfg(test)]
mod tests {
    use super::parse_input;

    #[test]
    fn parse_input_rejects_invalid_input() {}

    #[test]
    fn parse_input_handles_malformed_input() {
        let _result = parse_input("");
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /parser parse_input lacks invalid-input test evidence/u, output);
  assert.match(output, /parser parse_input lacks invalid\/empty\/oversized\/malformed test evidence/u, output);
});

test('invalid parser evidence requires an asserted rejection outcome', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_input(input: &str) -> Result<&str, &'static str> {
    if input.is_empty() { Err("invalid") } else { Ok(input) }
}

#[cfg(test)]
mod tests {
    use super::parse_input;

    #[test]
    fn parse_input_rejects_invalid_input() {
        assert!(parse_input("accepted").is_ok());
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /parser parse_input lacks invalid-input test evidence/u, output);
});

test('invalid parser evidence rejects an unrelated error assertion', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_input(input: &str) -> Result<&str, &'static str> {
    if input.is_empty() { Err("invalid") } else { Ok(input) }
}

#[cfg(test)]
mod tests {
    use super::parse_input;

    #[test]
    fn parse_input_rejects_invalid_input() {
        let _result = parse_input("accepted");
        assert!(Err::<(), ()>(()).is_err());
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /parser parse_input lacks invalid-input test evidence/u, output);
});

test('unsafe word inside a diagnostic string does not require unsafe proof', () => {
  const project = makeProject({
    'src/lib.rs': `
const DIAGNOSTIC: &str = "unsafe block requires a safety explanation";
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('tuple slice type annotations do not trigger unchecked indexing', () => {
  const project = makeProject({
    'src/lib.rs': `
fn resolve(items: &[(String, usize)]) -> Option<&usize> {
    items.iter().map(|(_, index)| index).next()
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-5\.3/u, output);
});

test('Option parser rejection evidence accepts is_none assertions', () => {
  const project = makeProject({
    'src/lib.rs': `
fn parse_optional(input: &str) -> Option<&str> {
    (!input.is_empty()).then_some(input)
}

#[cfg(test)]
mod tests {
    use super::parse_optional;

    #[test]
    fn parse_optional_rejects_empty_input() {
        assert!(parse_optional("").is_none());
    }

    #[test]
    fn parse_optional_handles_oversized_input() {
        assert!(parse_optional("accepted").is_some());
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.(?:16|17)/u, output);
});

test('conversion syntax inside a string literal does not require DTO evidence', () => {
  const project = makeProject({
    'src/lib.rs': `
const SOURCE: &str = "impl TryFrom<WidgetDto> for Widget {}";
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.18/u, output);
});

test('persisted DTO rejection evidence accepts a public boundary integration test', () => {
  const project = makeProject({
    'src/lib.rs': `
struct HubConfigResponse {
    hub: String,
    node_id: String,
}
struct HubConfig;
impl TryFrom<HubConfigResponse> for HubConfig {
    type Error = ();
    fn try_from(_: HubConfigResponse) -> Result<Self, Self::Error> { Err(()) }
}
`,
    'tests/api_integration.rs': `
#[test]
fn load_identity_rejects_invalid_persisted_hub_config() {
    let raw = r#"{"hub": " " , "nodeId": "node"}"#;
    let result: Result<(), ()> = Err(());
    let _error = result.expect_err("invalid persisted hub config must be rejected");
    assert!(raw.contains("hub"));
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.18/u, output);
});
