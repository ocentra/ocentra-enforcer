import fs from "node:fs";
import { finding, resolvePackRoot } from "../scripts/check-source-core.mjs";

const HARNESS_CONTRACT_SPECS = [
  ["HAR-2.1", "src/harness.mjs", [/\brunId\b/u, /\bcommand\b/u, /\bcwd\b/u, /\bstartedAt\b/u, /\bendedAt\b/u, /\bexitCode\b/u]],
  ["HAR-2.2", "src/harness.mjs", [/\bmaxArtifactBytes\b/u, /\bredactSecrets\b/u]],
  ["HAR-2.3", "src/harness.mjs", [/\bsortDiagnostics\b/u, /localeCompare/u]],
  ["HAR-2.4", "src/harness.mjs", [/\bparserDiagnostic\b/u, /HAR-2\.4/u]],
  ["HAR-2.5", "src/harness.mjs", [/\bcompiler-message\b/u, /\brustMessageToDiagnostic\b/u]],
  ["HAR-2.6", "src/harness.mjs", [/\bfilePath\b/u, /\bmessages\b/u, /eslint/u]],
  ["HAR-2.7", "src/harness.mjs", [/\bgeneralDiagnostics\b/u, /pyright/u, /ruff|mypy|pytest/u]],
  ["HAR-2.8", "src/harness.mjs", [/\bparsed\.runs\b/u, /\bSARIF result\b/u]],
  ["HAR-2.9", "src/harness.mjs", [/\bexport function lastFailure\b/u, /\brunDiagnostics\b/u]],
  ["HAR-2.10", "tests/enforcer-harness.test.mjs", [/Artifact path escapes harness root/u, /runs",\s*"artifact"/u]],
  ["HAR-2.11", "src/harness.mjs", [/\bpinned\b/u, /entry\.pinned === true/u]],
  ["HAR-2.12", "src/harness.mjs", [/\bok: exitCode === 0/u, /status: exitCode === 0 \? 'passed' : 'failed'/u]],
  ["HAR-2.13", "schemas/json/run-report.schema.json", [/"properties"/u]],
  ["HAR-2.14", "src/harness.mjs", [/\bredactSecrets\b/u, /\[REDACTED\]/u]],
  ["HAR-2.15", "src/harness.mjs", [/shell: false/u]],
];

const PROOF_CONTRACT_SPECS = [
  ["PROOF-1.1", "src/proof.mjs", [/\bprReady\b/u, /\bNo proof run found\b/u]],
  ["PROOF-1.2", "src/proof.mjs", [/\bgitState\b/u, /\bfiles\b/u, /\bprofile\b/u]],
  ["PROOF-1.3", "src/proof.mjs", [/manual-required/u, /manual-artifact/u]],
  ["PROOF-1.4", "src/proof.mjs", [/missing|required artifacts|failedArtifacts/u, /\bbyteLength\b/u]],
  ["PROOF-1.5", "src/proof.mjs", [/\bsha256\b/u, /hash-match|importedHashes|legacyHashes/u]],
  ["PROOF-1.6", "tests/enforcer-proof.test.mjs", [/dirty-worktree/u, /allowDirty/u]],
  ["PROOF-1.7", "src/proof.mjs", [/waived|unavailable|manual-required/u]],
  ["PROOF-1.8", "src/proof.mjs", [/command\.length === 0/u, /\bNo executable command\b/u]],
  ["PROOF-1.9", "src/proof.mjs", [/\bcommand:\s*\[/u, /shell: false/u]],
  ["PROOF-1.10", "proof/proofs.json", [/"docs"/u]],
  ["PROOF-1.11", "src/proof.mjs", [/\bcapabilities\b/u, /\bcapability\b/u]],
  ["PROOF-1.12", "src/proof.mjs", [/android-device|ios-device|manual-required/u]],
  ["PROOF-1.13", "src/proof.mjs", [/claimsProved/u, /claimsNotProved/u]],
  ["PROOF-1.14", "src/proof.mjs", [/diagnosticLimit/u, /slice\(0/u]],
  ["PROOF-1.15", "src/proof.mjs", [/\bredactSecrets\b/u, /\[REDACTED\]/u]],
];

const MCP_CONTRACT_SPECS = [
  ["MCP-1.1", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_scan/u, /ocentra_enforcer_check/u]],
  ["MCP-1.2", "mcp/rust-rules-mcp.mjs", [/decodeScanToolArguments/u, /decodeCheckToolArguments/u, /decodeCoordinationToolArguments/u]],
  ["MCP-1.3", "tests/rust-rules-mcp.test.mjs", [/unexpected argument/u, /result\.isError/u]],
  ["MCP-1.4", "mcp/rust-rules-mcp.mjs", [/summaryOnly/u, /includeScope/u]],
  ["MCP-1.5", "mcp/rust-rules-mcp.mjs", [/diagnosticLimit/u, /Math\.trunc\(args\.diagnosticLimit\)/u]],
  ["MCP-1.6", "mcp/rust-rules-mcp.mjs", [/shouldBlockStaleMcpTool/u, /COORDINATION_WRITE_TOOLS/u]],
  ["MCP-1.7", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_mcp_status/u, /buildMcpFingerprint/u]],
  ["MCP-1.8", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_explain/u, /runCli\("explain"/u]],
  ["MCP-1.9", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_route/u, /buildRouteReport/u]],
  ["MCP-1.10", "mcp/rust-rules-mcp.mjs", [/runCli\(decoded\.cargo \? "cargo" : "scan"/u, /read-only|scan/u]],
  ["MCP-1.11", "mcp/rust-rules-mcp.mjs", [/ocentra_enforcer_coordination_claim/u, /ocentra_enforcer_coordination_release/u]],
  ["MCP-1.12", "mcp/rust-rules-mcp.mjs", [/function toolError/u, /JSON\.stringify\(body/u]],
];

const SCANNER_CONTRACT_SPECS = [
  ["SCAN-1.1", "src/source-policy-scanners.mjs", [/maskJavaScriptLine/u]],
  ["SCAN-1.2", "src/source-policy-scanners.mjs", [/maskJavaScriptLine/u, /\/\/|\/\*/u]],
  ["SCAN-1.3", "src/generic-scanner-shared.mjs", [/ts-ignore|noqa|type:\s*ignore/u]],
  ["SCAN-1.4", "src/checks.mjs", [/split\(/u, /\\r\?\\n/u]],
  ["SCAN-1.5", "src/path-utils.mjs", [/toPosix/u, /normalizeRel/u]],
  ["SCAN-1.6", "src/path-utils.mjs", [/repoAbsolute/u, /path\.resolve/u]],
  ["SCAN-1.7", "src/path-utils.mjs", [/path\.isAbsolute/u, /path\.resolve/u]],
  ["SCAN-1.8", "src/path-utils.mjs", [/lstatSync/u, /isSymbolicLink/u]],
  ["SCAN-1.9", "src/path-utils.mjs", [/isSymbolicLink/u]],
  ["SCAN-1.10", "scripts/rust-rules.mjs", [/sortFindings/u, /compareFindings/u]],
  ["SCAN-1.11", "src/checks.mjs", [/maxArtifactBytes|64 \* 1024 \* 1024|maxBuffer/u]],
  ["SCAN-1.12", "src/generic-common-scanner.mjs", [/binary|readFileSync/u]],
  ["SCAN-1.13", "src/checks.mjs", [/try\s*\{/u, /catch/u]],
  ["SCAN-1.14", "src/routing.mjs", [/routeFamilyKeysForFile/u, /return \[\]/u]],
  ["SCAN-1.15", "scripts/rust-rules.mjs", [/--base/u, /--head/u]],
  ["SCAN-1.16", "src/checks.mjs", [/scopeEntries/u, /--files/u]],
  ["SCAN-1.17", "src/checks.mjs", [/mode: "all"|workspace/u]],
  ["SCAN-1.18", "scripts/rust-rules.mjs", [/Cargo\.toml/u, /package\.json/u]],
  ["SCAN-1.19", "scripts/rust-rules.mjs", [/scope/u, /files/u]],
  ["SCAN-1.20", "scripts/rust-rules.mjs", [/ignoreDirs/u, /ignoreFileGlobs/u]],
  ["SCAN-2.1", "scripts/rust-rules.mjs", [/cargo/u, /metadata/u]],
  ["SCAN-2.2", "scripts/rust-rules.mjs", [/scanRustFile/u, /signature|struct|enum/u]],
  ["SCAN-2.3", "src/harness.mjs", [/clippy|cargo/u, /compiler-message/u]],
  ["SCAN-2.4", "src/harness.mjs", [/rustdoc|cargo/u, /warning/u]],
  ["SCAN-2.5", "src/harness.mjs", [/eslint/u, /tsc/u]],
  ["SCAN-2.6", "src/generic-common-scanner.mjs", [/ruff/u, /output-format\\s\+json/u]],
  ["SCAN-2.7", "src/generic-common-scanner.mjs", [/pyright/u, /mypy/u]],
  ["SCAN-2.8", "src/harness.mjs", [/parsed\.runs/u, /SARIF/u]],
  ["SCAN-2.9", "src/generic-scanner-shared.mjs", [/RegExp|test\(/u]],
  ["SCAN-2.10", "src/harness.mjs", [/dedupeDiagnostics/u, /fingerprint/u]],
];

function collectRequiredPatternFindings(root, packRoot, specs) {
  const findings = [];
  for (const [ruleId, relFile, patterns] of specs) {
    const file = `${packRoot}/${relFile}`.replace(/\\/gu, "/");
    const text = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
    const missing = patterns.filter((pattern) => !pattern.test(text));
    if (missing.length === 0) continue;
    findings.push(
      finding(root, file, 1, ruleId, `${relFile} is missing contract marker(s): ${missing.map(String).join(", ")}`, null),
    );
  }
  return findings;
}

function collectHarnessContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), HARNESS_CONTRACT_SPECS);
}

function collectProofContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), PROOF_CONTRACT_SPECS);
}

function collectMcpContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), MCP_CONTRACT_SPECS);
}

function collectScannerContractFindings(root, args = {}) {
  return collectRequiredPatternFindings(root, resolvePackRoot(root, args), SCANNER_CONTRACT_SPECS);
}

export {
  collectHarnessContractFindings,
  collectProofContractFindings,
  collectMcpContractFindings,
  collectScannerContractFindings,
};
