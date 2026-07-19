import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import {
  DEFAULT_CONFIG,
  expectFailure,
  expectNoRule,
  makeProject,
  normalizeConfig,
  runGate,
  runScanner,
} from './rust-rules-test-support.mjs';
import {
  clearProofEvidenceCache,
  proofEvidenceCacheStats,
  resetProofEvidenceCacheStats,
} from '../scripts/rust-rules-source-test-evidence-cache.mjs';
import { hasRoundTripEvidence } from '../scripts/rust-rules-source-roundtrip-evidence.mjs';
test('constructor and property evidence bind to local definitions and registered property targets', () => {
  const externalOnly = makeProject({ 'src/lib.rs': 'pub fn use_external(input: &str) { let _ = input.parse::<u64>(); }\n' });
  expectNoRule(externalOnly, 'RR-12.16');
  const propertyCovered = makeProject({
    'src/lib.rs': 'pub fn parse_item(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }\n',
    'tests/property_parser_contracts.rs': 'proptest! { #[test] fn parser_contract(input in ".*") { let _ = parse_item(&input); } }\n',
  });
  expectNoRule(propertyCovered, 'RR-12.27');
  const missing = makeProject({ 'src/lib.rs': 'pub fn try_new(value: String) -> Result<(), ()> { let _ = value; Ok(()) }\n' });
  expectFailure(missing, 'RR-12.16');
});

test('parser rejection evidence is crate-wide and remains target-specific', () => {
  const covered = makeProject({
    'src/lib.rs': 'pub fn parse_lesson(input: &str) -> Result<(), ()> { if input.is_empty() { Err(()) } else { Ok(()) } }\n',
    'tests/lesson.rs': '#[test]\nfn rejects_invalid_lesson() { let error = parse_lesson("invalid").expect_err("invalid lesson must be rejected"); assert_eq!(error, ()); }\n',
  });
  expectNoRule(covered, 'RR-12.16');
  expectNoRule(covered, 'RR-12.17');

  const partiallyCovered = makeProject({
    'src/lib.rs': `
pub fn parse_one(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }
pub fn parse_two(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }
`,
    'tests/parser.rs': `
#[test]
fn rejects_invalid_parse_one() { assert!(parse_one("invalid").is_err()); }
proptest! { #[test] fn parse_one_property(input in ".*") { let _ = parse_one(&input); } }
`,
  });
  const result = runGate(partiallyCovered);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.notEqual(result.status, 0);
  assert.match(output, /RR-12\.16[\s\S]*parse_two/u);
  assert.match(output, /RR-12\.17[\s\S]*parse_two/u);
  assert.match(output, /RR-12\.27[\s\S]*parse_two/u);
  assert.doesNotMatch(output, /RR-12\.(?:16|17|27)[^\n]*parse_one lacks/u);

  const productionTextIsNotTestEvidence = makeProject({
    'src/lib.rs': 'pub fn parse_lesson(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }\n',
    'src/diagnostics.rs': 'fn describe_invalid_parse_lesson_rejection() -> &\'static str { "parse_lesson rejects invalid input" }\n',
  });
  expectFailure(productionTextIsNotTestEvidence, 'RR-12.16');
  expectFailure(productionTextIsNotTestEvidence, 'RR-12.17');
});

test('crate evidence cache refreshes after an external test changes in one process', () => {
  const project = makeProject({
    'src/lib.rs': 'pub fn parse_lesson(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }\n',
    'tests/lesson.rs': '#[test]\nfn lesson_smoke() { let _ = parse_lesson("valid"); }\n',
  });
  const sourcePath = path.join(project, 'src', 'lib.rs');
  const config = normalizeConfig(DEFAULT_CONFIG);
  const scope = { mode: 'files', files: [sourcePath] };
  const initial = runScanner(project, config, scope);
  assert.equal(initial.some((finding) => finding.ruleId === 'RR-12.16'), true);

  fs.writeFileSync(
    path.join(project, 'tests', 'lesson.rs'),
    '#[test]\nfn rejects_invalid_lesson_input() { let error = parse_lesson("invalid lesson input").expect_err("invalid lesson must be rejected"); assert_eq!(error, ()); }\n',
    'utf8',
  );
  const refreshed = runScanner(project, config, scope);
  assert.equal(refreshed.some((finding) => finding.ruleId === 'RR-12.16'), false);
  assert.equal(refreshed.some((finding) => finding.ruleId === 'RR-12.17'), false);
});

test('round-trip proof evidence builds one crate index per source snapshot and rebuilds after clear', () => {
  const project = makeProject({
    'src/boundary/proof.rs': `
#[derive(Serialize, Deserialize, PartialEq)]
pub struct FirstDto;
#[derive(Serialize, Deserialize, PartialEq)]
pub struct SecondDto;
#[cfg(test)]
mod tests {
    #[test]
    fn first_and_second_dto_round_trip() {
        let first = FirstDto;
        let first_wire = serde_json::to_string(&first).unwrap();
        let first_back = serde_json::from_str::<FirstDto>(&first_wire).unwrap();
        assert_eq!(first_back, first);
        let second = SecondDto;
        let second_wire = serde_json::to_string(&second).unwrap();
        let second_back = serde_json::from_str::<SecondDto>(&second_wire).unwrap();
        assert_eq!(second_back, second);
    }
}
`,
  });
  const sourcePath = path.join(project, 'src', 'boundary', 'proof.rs');
  const source = fs.readFileSync(sourcePath, 'utf8');

  resetProofEvidenceCacheStats();
  assert.equal(hasRoundTripEvidence(project, sourcePath, source, 'FirstDto'), true);
  assert.equal(hasRoundTripEvidence(project, sourcePath, source, 'SecondDto'), true);
  assert.equal(proofEvidenceCacheStats().indexBuilds, 1);

  clearProofEvidenceCache();
  assert.equal(hasRoundTripEvidence(project, sourcePath, source, 'FirstDto'), true);
  assert.equal(proofEvidenceCacheStats().indexBuilds, 2);
});

test('fuzz evidence binds to the binary parser target and not sibling parsers', () => {
  const covered = makeProject({
    'src/lib.rs': 'pub fn parse_packet(input: &[u8]) -> Result<(), ()> { let _packet = input; Ok(()) }\n',
    'fuzz/fuzz_targets/packet.rs': 'fn fuzz_parse_packet(bytes: &[u8]) { let _ = parse_packet(bytes); }\n',
  });
  expectNoRule(covered, 'RR-12.28');

  const sibling = makeProject({
    'src/lib.rs': `
pub fn parse_packet(input: &[u8]) -> Result<(), ()> { let _packet = input; Ok(()) }
pub fn parse_label(input: &str) -> Result<(), ()> { let _label = input; Ok(()) }
`,
  });
  const result = runGate(sibling);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /RR-12\.28[\s\S]*parse_packet/u);
  assert.doesNotMatch(output, /RR-12\.28[^\n]*parse_label/u);
});

test('DTO mapper entry points satisfy conversion evidence while missing conversion fails', () => {
  const passing = makeProject({
    'src/boundary/artifact_transport.rs': 'pub struct ArtifactTransportDto;\npub struct ArtifactTransport;\nimpl ArtifactTransportDto { fn into_domain(self) -> ArtifactTransport { ArtifactTransport } }\n',
  });
  expectNoRule(passing, 'RR-14.23');
  const failing = makeProject({
    'src/boundary/artifact_transport.rs': 'pub struct ArtifactTransportDto;\npub struct ArtifactTransport;\n',
  });
  expectFailure(failing, 'RR-14.23');

  const inlineRoundTripComment = makeProject({
    'src/boundary/proof.rs': `
#[derive(Serialize, Deserialize)]
pub struct ProofDto;

// ROUNDTRIP-TEST: comments are not evidence.
`,
  });
  expectFailure(inlineRoundTripComment, 'RR-14.25');

  const inlineRoundTripCovered = makeProject({
    'src/boundary/proof.rs': `
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ProofDto;
#[cfg(test)]
mod tests {
    #[test]
    fn proof_dto_round_trip_preserves_the_wire_shape() {
        let value = ProofDto;
        let wire = serde_json::to_string(&value).unwrap();
        let back = serde_json::from_str::<ProofDto>(&wire).unwrap();
        assert_eq!(back, value);
    }
}
`,
  });
  expectNoRule(inlineRoundTripCovered, 'RR-14.25');

  const externalRoundTripCovered = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': '#[test]\nfn proof_dto_round_trip_preserves_the_wire_shape() { let value = ProofDto; let wire = serde_json::to_string(&value).unwrap(); let back = serde_json::from_str::<ProofDto>(&wire).unwrap(); assert_eq!(back, value); }\n',
  });
  expectNoRule(externalRoundTripCovered, 'RR-14.25');

  const genericHelperCovered = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: T)
where T: Serialize + DeserializeOwned + PartialEq {
    let wire = serde_json::to_string(&value).unwrap();
    let restored: T = serde_json::from_str(&wire).unwrap();
    assert_eq!(restored, value);
}
#[test]
fn proof_dto_uses_the_verified_generic_round_trip_helper() {
    assert_json_round_trip::<ProofDto>(ProofDto);
}
`,
  });
  expectNoRule(genericHelperCovered, 'RR-14.25');

  const inferredGenericHelperCovered = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + PartialEq {
    let wire = serde_json::to_string(value).unwrap();
    let restored: T = serde_json::from_str(&wire).unwrap();
    assert_eq!(&restored, value);
}
#[test]
fn inferred_generic_helper_preserves_the_direct_projection() {
    let proof = ProofDto;
    assert_json_round_trip(&proof);
}
`,
  });
  expectNoRule(inferredGenericHelperCovered, 'RR-14.25');

  const directGenericHelperCovered = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + PartialEq {
    let wire = serde_json::to_string(value).unwrap();
    let restored: T = serde_json::from_str(&wire).unwrap();
    assert_eq!(&restored, value);
}
#[test]
fn direct_generic_helper_preserves_the_projection() {
    assert_json_round_trip(&ProofDto);
}
`,
  });
  expectNoRule(directGenericHelperCovered, 'RR-14.25');

  const inferredGenericHelperCoversNestedProjection = makeProject({
    'src/boundary/proof.rs': `
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ChildDto;
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ParentResponse { pub child: ChildDto }
`,
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + PartialEq {
    let wire = serde_json::to_string(value).unwrap();
    let restored: T = serde_json::from_str(&wire).unwrap();
    assert_eq!(&restored, value);
}
#[test]
fn inferred_generic_helper_preserves_the_nested_projection() {
    let response = ParentResponse { child: ChildDto };
    assert_json_round_trip(&response);
}
`,
  });
  expectNoRule(inferredGenericHelperCoversNestedProjection, 'RR-14.25');

  const fakeGenericHelper = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: T) { let _ = value; }
#[test]
fn proof_dto_calls_a_helper_without_wire_behavior() {
    assert_json_round_trip::<ProofDto>(ProofDto);
}
`,
  });
  expectFailure(fakeGenericHelper, 'RR-14.25');

  const inferredHelperWithMismatchedType = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned + PartialEq {
    let wire = serde_json::to_string(value).unwrap();
    let restored: T = serde_json::from_str(&wire).unwrap();
    assert_eq!(&restored, value);
}
#[test]
fn inferred_helper_cannot_use_a_mismatched_projection() {
    let other = OtherDto;
    assert_json_round_trip(&other);
}
`,
  });
  expectFailure(inferredHelperWithMismatchedType, 'RR-14.25');

  const inferredHelperWithoutEquality = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: &T)
where T: Serialize + DeserializeOwned {
    let wire = serde_json::to_string(value).unwrap();
    let _restored: T = serde_json::from_str(&wire).unwrap();
}
#[test]
fn inferred_helper_without_equality_is_not_evidence() {
    let proof = ProofDto;
    assert_json_round_trip(&proof);
}
`,
  });
  expectFailure(inferredHelperWithoutEquality, 'RR-14.25');

  const unrelatedGenericHelperBehavior = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': `
fn assert_json_round_trip<T>(value: T)
where T: Serialize + DeserializeOwned + PartialEq {
    let unrelated = OtherDto;
    let wire = serde_json::to_string(&unrelated).unwrap();
    let restored: OtherDto = serde_json::from_str(&wire).unwrap();
    assert_eq!(restored, unrelated);
    let _ = value;
}
#[test]
fn proof_dto_cannot_borrow_unrelated_helper_behavior() {
    assert_json_round_trip::<ProofDto>(ProofDto);
}
`,
  });
  expectFailure(unrelatedGenericHelperBehavior, 'RR-14.25');

  const externalCommentOnly = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/unrelated_roundtrip.rs': '// ProofDto is not exercised by this test.\n#[test]\nfn unrelated_round_trip() { let value = OtherDto; let wire = serde_json::to_string(&value).unwrap(); let back = serde_json::from_str::<OtherDto>(&wire).unwrap(); assert_eq!(back, value); }\n',
  });
  expectFailure(externalCommentOnly, 'RR-14.25');

  const unrelatedTypedMention = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/unrelated_roundtrip.rs': `
#[test]
fn unrelated_round_trip() {
    let proof: Option<ProofDto> = None;
    let value = OtherDto;
    let wire = serde_json::to_string(&value).unwrap();
    let back = serde_json::from_str::<OtherDto>(&wire).unwrap();
    assert_eq!(back, value);
    assert_eq!(proof, None);
}
`,
  });
  expectFailure(unrelatedTypedMention, 'RR-14.25');

  const nestedAggregateCovered = makeProject({
    'src/boundary/proof.rs': `
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ChildDto;
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ParentResponse { pub child: ChildDto }
#[cfg(test)]
mod tests {
    #[test]
    fn parent_response_round_trip_covers_nested_transport_types() {
        let value = ParentResponse { child: ChildDto };
        let wire = serde_json::to_string(&value).unwrap();
        let back: ParentResponse = serde_json::from_str(&wire).unwrap();
        assert_eq!(back, value, "aggregate wire identity must be preserved");
    }
}
`,
  });
  expectNoRule(nestedAggregateCovered, 'RR-14.25');

  const crossFileAggregateCovered = makeProject({
    'src/boundary/child.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ChildDto;\n',
    'src/boundary/parent.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ParentResponse { pub child: ChildDto }\n',
    'tests/parent_roundtrip.rs': `
#[test]
fn parent_response_round_trip_covers_cross_file_nested_transport_types() {
    let payload = build_parent_response();
    let wire = serde_json::to_string(&payload).unwrap();
    let restored: ParentResponse = serde_json::from_str(&wire).unwrap();
    assert_eq!(restored, payload);
}
`,
  });
  expectNoRule(crossFileAggregateCovered, 'RR-14.25');

  const mutableEvidence = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize, PartialEq)]\npub struct ProofDto;\n',
    'tests/proof_roundtrip.rs': '#[test]\nfn proof_dto_round_trip_preserves_the_wire_shape() { let value = ProofDto; let wire = serde_json::to_string(&value).unwrap(); let back = serde_json::from_str::<ProofDto>(&wire).unwrap(); assert_eq!(back, value); }\n',
  });
  const mutableConfig = normalizeConfig(DEFAULT_CONFIG);
  const mutableScope = {
    mode: 'files',
    files: [
      path.join(mutableEvidence, 'src', 'boundary', 'proof.rs'),
      path.join(mutableEvidence, 'tests', 'proof_roundtrip.rs'),
    ],
  };
  let mutableFindings = runScanner(mutableEvidence, mutableConfig, mutableScope);
  assert.equal(mutableFindings.some((finding) => finding.ruleId === 'RR-14.25'), false);
  fs.writeFileSync(
    path.join(mutableEvidence, 'tests', 'proof_roundtrip.rs'),
    '// ProofDto evidence was removed between top-level scans.\n',
  );
  mutableFindings = runScanner(mutableEvidence, mutableConfig, mutableScope);
  assert.equal(
    mutableFindings.some((finding) => finding.ruleId === 'RR-14.25'),
    true,
    'external evidence cache must be invalidated at the start of every scan',
  );

  const roundTripMissing = makeProject({
    'src/boundary/proof.rs': '#[derive(Serialize, Deserialize)]\npub struct ProofDto;\n',
  });
  expectFailure(roundTripMissing, 'RR-14.25');
});

test('test-only DTO fixtures do not require product round-trip evidence', () => {
  const project = makeProject({
    'tests/tool_diff.rs': `
#[derive(Serialize, Deserialize)]
pub struct ToolDiffRowDto;
`,
  });
  const findings = runScanner(project, normalizeConfig(DEFAULT_CONFIG), {
    mode: 'files',
    files: [path.join(project, 'tests', 'tool_diff.rs')],
  });
  assert.equal(
    findings.some((finding) => finding.ruleId === 'RR-14.25'),
    false,
  );
});

test('external negative conversion evidence is target-specific', () => {
  const covered = makeProject({
    'src/boundary/session.rs': 'pub struct SessionDto;\npub struct Session;\nimpl TryFrom<SessionDto> for Session { type Error = (); fn try_from(value: SessionDto) -> Result<Self, Self::Error> { let _ = value; Ok(Session) } }\n',
    'tests/session_boundary.rs': '#[test]\nfn session_dto_rejects_empty_command() { let error = Session::try_from(SessionDto).expect_err("reject"); assert_eq!(error, ()); }\n',
  });
  expectNoRule(covered, 'RR-12.18');
  const missing = makeProject({
    'src/boundary/session.rs': 'pub struct SessionDto;\npub struct Session;\nimpl TryFrom<SessionDto> for Session { type Error = (); fn try_from(value: SessionDto) -> Result<Self, Self::Error> { let _ = value; Ok(Session) } }\n',
  });
  expectFailure(missing, 'RR-12.18');
});

test('snapshot rule requires a snapshot assertion rather than a section comment or local snapshot variable', () => {
  const passing = makeProject({
    'tests/snapshot_sections.rs': `
#[test]
fn export_section_uses_a_snapshot_value() {
    // 4: personal-scope export -> import roundtrip.
    let snapshot = "2026-07-05";
    assert!(!snapshot.is_empty());
}
`,
  });
  expectNoRule(passing, 'RR-12.26');
  const failing = makeProject({
    'src/snapshot_test.rs': `
#[test]
fn snapshot_has_uuid() {
    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    toMatchSnapshot();
}
`,
  });
  expectFailure(failing, 'RR-12.26');
});
