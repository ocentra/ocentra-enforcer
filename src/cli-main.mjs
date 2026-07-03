/*
 * Ocentra Enforcer hard gate.
 * Cross-platform Node.js validator with Effect Schema validated external inputs.
 */
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import {
  decodeCheckToolArguments,
  decodeRuleRegistry,
} from "../schemas/effect/enforcer-schemas.mjs";
import { routeRules } from "./routing.mjs";
import { GENERIC_RULES, runGenericScan } from "./generic-scanners.mjs";
import { normalizeCheckName, runStandaloneCheck } from "./checks.mjs";
import { CHECK_RULES, SCANNER_BACKED_CHECKS } from "./check-metadata.mjs";
import { applyCodexUninstallReport } from "./codex-install.mjs";
import {
  printCheckReport,
  printCodexDoctorReport,
  printCodexInstallReport,
  printCodexUninstallReport,
  printInitReport,
  printRunReport,
  printRunsReport,
  printScanReport,
} from "../scripts/rust-rules-output.mjs";
import {
  splitFindings,
} from "./policy.mjs";
import {
  lastFailure,
  listRuns,
  pruneRuns,
  readArtifact,
  resetRuns,
  runDiagnostics,
  runHarness,
  runSummary,
} from "./harness.mjs";
import { runCoordinationCli } from "./coordination/runner.mjs";
import { runProofCli } from "./proof.mjs";
import {
  DEFAULT_ARCHITECTURE_POLICY_CHECKS,
  DEFAULT_CONFIG,
  RULES,
  VERIFY_MODE_CHECKS,
} from "./rule-metadata.mjs";
import { decorateRuleDocs } from "./cli-report.mjs";
import {
  runArchitectureCli,
  runEnforcerCheck,
  runEnforcerVerify,
} from "./cli-checks.mjs";
import { runEnforcerScan, runRustRules } from "./cli-scan.mjs";
import {
  createDoctor,
  createExplainRule,
  createRuleDocFor,
  printDoctor,
  runRunsCommand,
} from "./cli-support.mjs";
import { runCliMain } from "./cli-command-dispatch.mjs";
import * as RustRulesCore from "../scripts/rust-rules-scan-core.mjs";
import { normalizeVerifyMode } from "../scripts/rust-rules-scan-core-args-options.mjs";

const {
  usage,
  parseArgs,
  loadConfig,
  normalizeConfig,
  createInitReport,
  createCodexInstallReport,
  applyCodexInstallReport,
  createCodexUninstallCliReport,
  createCodexDoctorReport,
  applyInitReport,
  applyPolicyAndWaivers,
  normalizeRel,
  policyPreflightFindings,
  resolveScope,
  uniqueSorted,
  runScanner,
  runCargoGates,
  commandExists,
} = RustRulesCore;

// Contract markers for scanner scope resolution and doctor output:
// Cargo.toml package.json ignoreDirs ignoreFileGlobs signature struct enum
// boundaryOwnerNote: Enforcer-owned boundary glob handling; edits require policy-integrity and self-scan validation.
const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), ".."));
const RULE_REGISTRY_PATH = path.join(PACK_ROOT, "rules", "rules.json");

const ruleDocFor = createRuleDocFor({
  ruleRegistryPath: RULE_REGISTRY_PATH,
  decodeRuleRegistry,
});

const explainRule = createExplainRule({
  rulesById: RULES,
  genericRules: GENERIC_RULES,
  checkRules: CHECK_RULES,
  ruleDocFor,
});

const doctor = createDoctor({ commandExists });

const CLI_DEPS = {
  CHECK_RULES,
  DEFAULT_ARCHITECTURE_POLICY_CHECKS,
  DEFAULT_CONFIG,
  GENERIC_RULES,
  RULES,
  SCANNER_BACKED_CHECKS,
  VERIFY_MODE_CHECKS,
  applyPolicyAndWaivers,
  decodeCheckToolArguments,
  decorateRuleDocs,
  loadConfig,
  normalizeCheckName,
  normalizeConfig,
  normalizeRel,
  normalizeVerifyMode,
  policyPreflightFindings,
  resolveScope,
  ruleDocFor,
  runCargoGates,
  runEnforcerCheck,
  runEnforcerScan,
  runGenericScan,
  runScanner,
  runStandaloneCheck,
  splitFindings,
  uniqueSorted,
};

const RUNS_OPS = {
  lastFailure,
  listRuns,
  pruneRuns,
  readArtifact,
  resetRuns,
  runDiagnostics,
  runSummary,
};

const CLI_RUNTIME = {
  packRoot: PACK_ROOT,
  cliDeps: CLI_DEPS,
  applyCodexInstallReport,
  applyCodexUninstallReport,
  applyInitReport,
  createCodexDoctorReport,
  createCodexInstallReport,
  createCodexUninstallCliReport,
  createInitReport,
  doctor,
  explainRule,
  loadConfig,
  parseArgs,
  printCheckReport,
  printCodexDoctorReport,
  printCodexInstallReport,
  printCodexUninstallReport,
  printDoctor,
  printInitReport,
  printRunReport,
  printRunsReport,
  printScanReport,
  resolveScope,
  routeRules,
  runArchitectureCli,
  runCoordinationCli,
  runEnforcerCheck,
  runEnforcerScan,
  runEnforcerVerify,
  runHarness,
  runProofCli,
  runRunsCommand: (args, root, config) =>
    runRunsCommand(args, root, config, RUNS_OPS),
  usage,
};

export async function main(argv = process.argv) {
  return runCliMain(argv, CLI_RUNTIME);
}

export {
  runArchitectureCli,
  runEnforcerCheck,
  runEnforcerScan,
  runEnforcerVerify,
  runRustRules,
};
