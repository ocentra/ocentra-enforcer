import test from "node:test";
import assert from "node:assert/strict";
import {
  expectFailure,
  expectNoRule,
  makeProject,
} from "./rust-rules-test-support.mjs";

test('test-structure rules use balanced masked bodies', () => {
  const passing = makeProject({ 'tests/fixture.rs': `
#[test]
fn fixture_text_is_not_a_test() {
    let fixture = r#"#[test]\nfn quoted_fixture() {}"#;
    let panic_attribute = "#[should_panic]";
    let bytes = b"fn byte_fixture() { 1 }";
    assert!(!fixture.is_empty());
    assert!(!panic_attribute.is_empty());
    assert!(!bytes.is_empty());
}

#[test]
fn following_test_remains_visible() {
    assert_eq!(2 + 2, 4);
}
` });
  expectNoRule(passing, 'RR-12.24');
  expectNoRule(passing, 'RR-12.20');
  const failing = makeProject({ 'src/lib.rs': '#[test]\nfn empty() {}\n' });
  expectFailure(failing, 'RR-12.24');
});

test('construction-only tests accept delegated proof helpers but still reject construction alone', () => {
  const delegatedProof = makeProject({ 'src/lib.rs': `
#[test]
fn validator_fixture_has_behavioral_proof() -> Result<(), ()> {
    let validator = Validator::new()?;
    run_fixture_parity(&validator)?;
    Ok(())
}
` });
  expectNoRule(delegatedProof, 'RR-12.25');

  const manifestDelegatedProof = makeProject({ 'src/lib.rs': `
#[test]
fn validator_manifest_has_behavioral_proof() -> Result<(), ()> {
    let validator = Validator::new()?;
    run_manifest_fixture_parity(&validator, "bad", "good")?;
    Ok(())
}
` });
  expectNoRule(manifestDelegatedProof, 'RR-12.25');

  const propertyAssertion = makeProject({ 'src/lib.rs': `
proptest! {
    #[test]
    fn constructor_property(value in 1_u8..10) {
        let outcome = Validator::new(value);
        prop_assert!(outcome.is_ok());
    }
}
` });
  expectNoRule(propertyAssertion, 'RR-12.25');

  const constructionOnly = makeProject({ 'src/lib.rs': `
#[test]
fn validator_constructor_only() {
    let _validator = Validator::new();
}
` });
  expectFailure(constructionOnly, 'RR-12.25');
});
