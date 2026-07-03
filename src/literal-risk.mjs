import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { normalizeRel } from "./path-utils.mjs";
import { buildLiteralRiskScanCommand } from "./literal-risk-command.mjs";
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

function builtBinaryCandidates() {
  return [
    path.join(LITERAL_SCAN_TARGET, "debug", BINARY_NAME),
    path.join(LITERAL_SCAN_TARGET, "release", BINARY_NAME),
  ];
}

function resolveBuiltBinary() {
  return builtBinaryCandidates().find((candidate) => fs.existsSync(candidate));
}

function ensureBuiltBinary() {
  const existingBinary = resolveBuiltBinary();
  if (existingBinary) return existingBinary;
  if (!fs.existsSync(LITERAL_SCAN_MANIFEST)) {
    throw new Error(
      `Literal-risk scanner crate is missing: ${LITERAL_SCAN_MANIFEST}`,
    );
  }
  const build = spawnSync(
    "cargo",
    [
      "build",
      "--quiet",
      "--manifest-path",
      LITERAL_SCAN_MANIFEST,
      "--bin",
      "ocentra-literal-scan",
    ],
    {
      cwd: LITERAL_SCAN_ROOT,
      encoding: "utf8",
      shell: false,
      maxBuffer: 64 * 1024 * 1024,
    },
  );
  if (build.error) {
    throw new Error(
      `literal-risk scanner build failed to start: ${build.error.message}`,
    );
  }
  if (build.status !== 0) {
    throw new Error(
      `literal-risk scanner build failed with exit ${build.status ?? "unknown"}.\n${build.stdout ?? ""}\n${build.stderr ?? ""}`.trim(),
    );
  }
  const builtBinary = resolveBuiltBinary();
  if (!builtBinary) {
    throw new Error(
      `literal-risk scanner build completed but produced no binary under ${LITERAL_SCAN_TARGET}`,
    );
  }
  return builtBinary;
}

function resolveScannerInvocation() {
  return {
    command: ensureBuiltBinary(),
    args: [],
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
  const command = buildLiteralRiskScanCommand(
    invocation,
    rootPath,
    scannerOptions,
    explicitFiles,
  );
  const result = spawnSync(invocation.command, command, {
    cwd: rootPath,
    encoding: "utf8",
    shell: false,
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) {
    throw new Error(
      `literal-risk scanner failed to start: ${result.error.message}`,
    );
  }
  const stdout = result.stdout ?? "";
  const stderr = result.stderr ?? "";
  let report;
  try {
    report = JSON.parse(stdout || "{}");
  } catch (error) {
    throw new Error(
      `literal-risk scanner emitted invalid JSON: ${error.message}\n${stdout}\n${stderr}`,
    );
  }
  return {
    ok: result.status === 0 && report.ok === true,
    status: result.status,
    root: rootPath,
    files: explicitFiles,
    options: scannerOptions,
    report,
    stderr,
  };
}

export function mapLiteralRiskFindings(scan, root, options = {}) {
  return mapLiteralRiskFindingsImpl(scan, root, options);
}
