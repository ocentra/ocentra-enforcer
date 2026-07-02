import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import {
  CLI_PATH,
  compactScope,
  countBy,
  maybeCompactReport,
  parseJson,
  uniqueSorted,
} from "./rust-rules-mcp-helpers.mjs";
import { buildCliInvocation } from "./rust-rules-mcp-runner-cli.mjs";

const validationHistory = new Map();

export function runCli(command, args) {
  if (command === "explain") {
    return runCliProcess(
      [CLI_PATH, "explain", args.ruleId, "--json"],
      process.cwd(),
      command,
      args,
    );
  }
  const { cliArgs, root } = buildCliInvocation(command, args);
  return runCliProcess(cliArgs, root, command, args);
}

export function latestValidationSummary(args = {}) {
  const root = path.resolve(args.root ?? process.cwd()).toLowerCase();
  const entries = validationHistory.get(root) ?? [];
  if (args.tool === "check") {
    return entries.find((entry) => entry.kind === "check") ?? null;
  }
  if (args.tool === "scan") {
    return entries.find((entry) => entry.kind === "scan") ?? null;
  }
  return entries[0] ?? null;
}

function runCliProcess(cliArgs, cwd, command = null, args = {}) {
  const result = spawnSync(process.execPath, cliArgs, {
    cwd,
    encoding: "utf8",
    shell: false,
  });

  const stdout = result.stdout?.trim() ?? "";
  const stderr = result.stderr?.trim() ?? "";
  const parsed = parseJson(stdout);
  const report =
    parsed && (command === "scan" || command === "cargo" || command === "check")
      ? maybeCompactReport(parsed, args)
      : parsed;
  if (
    parsed &&
    (command === "scan" || command === "cargo" || command === "check")
  ) {
    recordValidationReport(command, parsed, args);
  }
  const text =
    report != null
      ? JSON.stringify(report, null, 2)
      : stdout ||
        JSON.stringify({ ok: false, status: result.status, stderr }, null, 2);
  return {
    isError: (result.status ?? 1) !== 0,
    content: [{ type: "text", text }],
  };
}

function recordValidationReport(command, report, args) {
  const root = path.resolve(args.root ?? report.root ?? process.cwd());
  const key = root.toLowerCase();
  const findings = [...(report.violations ?? []), ...(report.warnings ?? [])];
  const summary = {
    kind: command === "check" ? "check" : "scan",
    command: report.command,
    check: report.check,
    ok: report.ok,
    root,
    profileName: report.profileName,
    at: new Date().toISOString(),
    bySeverity: report.bySeverity ?? countBy(findings, "severity"),
    counts: {
      findings: findings.length,
      violations: report.violations?.length ?? 0,
      warnings: report.warnings?.length ?? 0,
    },
    ruleIds: uniqueSorted(findings.map((finding) => finding.ruleId)),
    docs: uniqueSorted(findings.map((finding) => finding.doc).filter(Boolean)),
    scope: compactScope(report.scope),
  };
  const entries = validationHistory.get(key) ?? [];
  entries.unshift(summary);
  validationHistory.set(key, entries.slice(0, 20));
}
