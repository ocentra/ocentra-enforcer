import { addViolation, contextHas } from "./rust-rules-path-core.mjs";
import { hasDependencyJustification } from "./rust-rules-cargo-manifest-identity.mjs";

/** Applies dependency policy checks to a parsed Cargo manifest context. */
export function scanManifestDependencyPolicy(context) {
  addWorkspaceMemberFinding(context);
  addDependencyJustificationFinding(context);
  addTestOnlyFindings(context);
  addProcMacroFinding(context);
  addNativeFinding(context);
  addDefaultFeatureFinding(context);
}

function addWorkspaceMemberFinding({ violations, root, manifest, lineNo, line, dependencyName, inProductionDependencySection, workspacePackageNames, currentPackageName }) {
  if (inProductionDependencySection && workspacePackageNames.has(dependencyName) && dependencyName !== currentPackageName && !/\b(?:path|workspace)\s*=/u.test(line)) addViolation(violations, root, manifest, lineNo, "RR-9.26", `${dependencyName} is a workspace member but is not linked by path/workspace dependency syntax.`, line);
}

function addDependencyJustificationFinding({ violations, root, manifest, lineNo, line, dependencyName, lines, index, workspaceDependencyJustification }) {
  const inherited = /\bworkspace\s*=\s*true\b/u.test(line) && workspaceDependencyJustification.get(dependencyName) === true;
  if (!inherited && !hasDependencyJustification(lines, index)) addViolation(violations, root, manifest, lineNo, "RR-9.18", `${dependencyName} lacks DEPENDENCY-JUSTIFICATION.`, line);
}

function addTestOnlyFindings({ violations, root, manifest, lineNo, line, dependencyName, inProductionDependencySection, config }) {
  if (inProductionDependencySection && config.testOnlyCratesSet.has(dependencyName)) ["RR-9.27", "RR-9.28", "RR-9.29"].forEach((ruleId) => addViolation(violations, root, manifest, lineNo, ruleId, testOnlyDetail(ruleId, dependencyName), line));
}

function testOnlyDetail(ruleId, dependencyName) {
  return ruleId === "RR-9.27" ? `${dependencyName} is test-only but appears in production dependencies.` : ruleId === "RR-9.28" ? `${dependencyName} must be in dev-dependencies only.` : `${dependencyName} must not be a production dependency in a runtime crate.`;
}

function addProcMacroFinding({ violations, root, manifest, lineNo, line, dependencyName, inProductionDependencySection, lines, index }) {
  if (inProductionDependencySection && /\b(?:syn|quote|proc-macro2|darling|proc-macro-error)\b/u.test(dependencyName) && !contextHas(lines, index, "PROC-MACRO-DEPENDENCY-JUSTIFICATION:", 4)) addViolation(violations, root, manifest, lineNo, "RR-9.20", `${dependencyName} is a proc-macro ecosystem dependency in runtime dependencies without approval.`, line);
}

function addNativeFinding({ violations, root, manifest, lineNo, line, dependencyName, lines, index }) {
  if (/^(?:openssl|openssl-sys|libsqlite3-sys|ring|rusqlite|bindgen|cc|cmake|pkg-config)$/u.test(dependencyName) && !contextHas(lines, index, "NATIVE-DEPENDENCY-JUSTIFICATION:", 4)) addViolation(violations, root, manifest, lineNo, "RR-9.21", `${dependencyName} is native/build-linked and lacks NATIVE-DEPENDENCY-JUSTIFICATION.`, line);
}

function addDefaultFeatureFinding({ violations, root, manifest, lineNo, line, dependencyName }) {
  if (/\b(?:tokio|reqwest|sqlx|diesel|aws-sdk|openssl|rusqlite)\b/u.test(dependencyName) && /\{[^}]*version\s*=/u.test(line) && !/\bdefault-features\s*=\s*false\b/u.test(line)) addViolation(violations, root, manifest, lineNo, "RR-9.17", `${dependencyName} must set default-features explicitly to false or document the default feature policy.`, line);
}
