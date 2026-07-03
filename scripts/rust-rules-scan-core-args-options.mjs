import { normalizeCheckName } from "../src/checks.mjs";

const VERIFY_MODE_CHECKS = { fast: true, local: true, ci: true, parent: true };

export function parseAdapterList(value) {
  return String(value ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

export function parseFileList(value) {
  return String(value ?? "")
    .split(/[,\r\n]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

export function normalizeVerifyMode(value) {
  const mode = String(value ?? "local").trim().toLowerCase();
  if (mode === "") return "local";
  if (!Object.hasOwn(VERIFY_MODE_CHECKS, mode)) {
    throw new Error(`Unknown verify mode: ${value}`);
  }
  return mode;
}

export function pushListValue(args, key, value) {
  if (!Array.isArray(args[key])) args[key] = [];
  args[key].push(value);
}

export const FLAG_OPTIONS = {
  "--staged": "staged",
  "--tracked": "tracked",
  "--strict-empty-test-trees": "strictEmptyTestTrees",
  "--workspace": "workspace",
  "--all": "workspace",
  "--no-skill": "installSkill",
  "--no-global-agents": "installGlobalAgents",
  "--dry-run": "dryRun",
  "--force": "force",
  "--scan-only": "scanOnly",
  "--json": "json",
  "--include-low": "literalRiskIncludeLow",
  "--include-ignored": "literalRiskIncludeIgnored",
  "--include-unknown-code": "literalRiskIncludeUnknownCode",
  "--no-respect-gitignore": "literalRiskRespectGitignore",
  "--help": "help",
  "-h": "help",
};

export const VALUE_OPTIONS = {
  "--root": (args, value) => {
    args.root = value;
    args.rootExplicit = true;
  },
  "--config": (args, value) => {
    args.configPath = value;
  },
  "--profile": (args, value) => {
    args.profile = value;
  },
  "--verify-mode": (args, value) => {
    args.verifyMode = normalizeVerifyMode(value);
  },
  "--languages": (args, value) => {
    args.languages = parseAdapterList(value);
  },
  "--adapters": (args, value) => {
    args.adapters = parseAdapterList(value);
  },
  "--tool": (args, value) => {
    args.runTool = value;
  },
  "--run-id": (args, value) => {
    args.runId = value;
  },
  "--rule-id": (args, value) => {
    args.routeRuleId = value;
  },
  "--check-config": (args, value) => {
    args.checkConfigPath = value;
  },
  "--output": (args, value) => {
    args.output = value;
  },
  "--codex-config": (args, value) => {
    args.codexConfigPath = value;
  },
  "--ledger-root": (args, value) => {
    args.ledgerRoot = value;
  },
  "--server-name": (args, value) => {
    args.mcpServerName = value;
  },
  "--artifact": (args, value) => {
    args.artifact = value;
  },
  "--limit": (args, value) => {
    args.limit = Number(value);
  },
  "--limit-bytes": (args, value) => {
    args.limitBytes = Number(value);
  },
  "--severity": (args, value) => {
    args.severity = value;
  },
  "--status": (args, value) => {
    args.status = value;
  },
  "--file": (args, value) => {
    args.file = value;
  },
  "--tag": (args, value) => {
    args.tag = value;
  },
  "--domain": (args, value) => {
    args.domain = value;
  },
  "--package-name": (args, value) => {
    args.packageName = value;
  },
  "--crate-name": (args, value) => {
    args.crateName = value;
  },
  "--crate": (args, value) => {
    args.crateName = value;
  },
  "--min-score": (args, value) => {
    args.literalRiskMinScore = Number(value);
  },
  "--max-file-bytes": (args, value) => {
    args.literalRiskMaxFileBytes = Number(value);
  },
  "--fail-above": (args, value) => {
    args.literalRiskFailAbove = Number(value);
  },
  "--hard-category": (args, value) => {
    pushListValue(args, "literalRiskHardCategories", value);
  },
  "--hard-rule-id": (args, value) => {
    pushListValue(args, "literalRiskHardRuleIds", value);
  },
};
