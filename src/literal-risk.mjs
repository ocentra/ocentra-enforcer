import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { normalizeRel } from "./path-utils.mjs";
import { buildLiteralRiskOptions } from "./literal-risk-options.mjs";
import { mapLiteralRiskFindings as mapLiteralRiskFindingsImpl } from "./literal-risk-findings.mjs";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), ".."));
const LITERAL_SCAN_ROOT = path.join(PACK_ROOT, "Tools", "ocentra-literal-scan");
const LITERAL_SCAN_MANIFEST = path.join(LITERAL_SCAN_ROOT, "Cargo.toml");
const LITERAL_SCAN_TARGET = path.join(LITERAL_SCAN_ROOT, "target");
const BINARY_NAME =
  process.platform === "win32"
    ? "ocentra-literal-scan.exe"
    : "ocentra-literal-scan";

function resolveScannerInvocation() {
  const builtBinary = [
    path.join(LITERAL_SCAN_TARGET, "debug", BINARY_NAME),
    path.join(LITERAL_SCAN_TARGET, "release", BINARY_NAME),
  ].find((candidate) => fs.existsSync(candidate));
  if (builtBinary) {
    return { command: builtBinary, args: [] };
  }
  if (!fs.existsSync(LITERAL_SCAN_MANIFEST)) {
    throw new Error(
      `Literal-risk scanner crate is missing: ${LITERAL_SCAN_MANIFEST}`,
    );
  }
  return {
    command: "cargo",
    args: ["run", "--manifest-path", LITERAL_SCAN_MANIFEST, "--"],
  };
}

export function runLiteralRiskScan({
  root,
  files = [],
  config = {},
  args = {},
}) {
  const rootPath = path.resolve(root ?? process.cwd());
  const scannerOptions = buildLiteralRiskOptions(config, args);
  const invocation = resolveScannerInvocation();
  const explicitFiles = files
    .map((file) => normalizeRel(rootPath, file))
    .filter(Boolean);
  const command = [
    ...invocation.args,
    "scan",
    "--root",
    rootPath,
    "--json",
    "--min-score",
    String(scannerOptions.minScore),
  ];
  if (scannerOptions.includeLow) command.push("--include-low");
  if (scannerOptions.includeIgnored) command.push("--include-ignored");
  if (scannerOptions.includeUnknownCode) command.push("--include-unknown-code");
  if (!scannerOptions.respectGitignore) command.push("--no-respect-gitignore");
  if (scannerOptions.failAbove !== null && scannerOptions.failAbove !== undefined) {
    command.push("--fail-above", String(scannerOptions.failAbove));
  }
  if (scannerOptions.maxFileBytes !== null && scannerOptions.maxFileBytes !== undefined) {
    command.push("--max-file-bytes", String(scannerOptions.maxFileBytes));
  }
  if (explicitFiles.length > 0) command.push("--files", ...explicitFiles);
  const result = spawnSync(invocation.command, command, {
    cwd: rootPath,
    encoding: "utf8",
    shell: false,
    maxBuffer: 64 * 1024 * 1024,
  });
  let report;
  try {
    report = JSON.parse(result.stdout || "{}");
  } catch (error) {
    throw new Error(
      `literal-risk scanner emitted invalid JSON: ${error.message}\n${result.stdout}\n${result.stderr}`,
    );
  }
  return {
    ok: result.status === 0 && report.ok === true,
    status: result.status,
    root: rootPath,
    files: explicitFiles,
    options: scannerOptions,
    report,
    stderr: result.stderr,
  };
}

export function mapLiteralRiskFindings(scan, root, options = {}) {
  return mapLiteralRiskFindingsImpl(scan, root, options);
}
