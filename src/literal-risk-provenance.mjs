const DETECTOR_DEFINITION_FILES = new Set([
  "crates/enforcer-core/src/boundary/redaction.rs",
  "crates/enforcer-lang-security/src/boundary/spec.rs",
  "crates/enforcer-memory/src/redaction.rs",
  "src/harness.mjs",
]);

/** Classifies a literal-risk finding by the provenance of its source file. */
export function classifyLiteralRiskProvenance(file) {
  const rel = String(file ?? "").replaceAll("\\", "/");
  if (/^vendor\//u.test(rel) || /\/vendor\//u.test(rel)) return "vendored";
  if (/^src\/coordination\/vendor\//u.test(rel)) return "vendored";
  if (/^profiles\/[^/]+\/legacy-scripts\//u.test(rel)) return "packaged-profile";
  if (/^(?:proof|output|test-results)\//u.test(rel) || /\/proof\//u.test(rel)) {
    return "proof-artifact";
  }
  if (/^(?:tests?|fixtures?)\//u.test(rel) || /\/(?:tests?|fixtures?)\//u.test(rel)) {
    return "test-fixture";
  }
  if (/^crates\/[^/]+\/src\/(?:lib_)?tests?\.rs$/u.test(rel)) return "test-fixture";
  if (
    /^crates\/enforcer-literal-scan\/src\/boundary\/risk_/u.test(rel) ||
    /^crates\/enforcer-lang-[^/]+\/src\/rules\//u.test(rel) ||
    /^crates\/enforcer-security\/src\/(?:boundary\/)?rules\//u.test(rel) ||
    /^scripts\/(?:rust-rules-source-|check-source-core-)/u.test(rel) ||
    /^src\/(?:source-policy-|generic-(?:scanner|common)|rule-metadata)/u.test(rel) ||
    DETECTOR_DEFINITION_FILES.has(rel)
  ) {
    return "detector-definition";
  }
  return "first-party";
}
