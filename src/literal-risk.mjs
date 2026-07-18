import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { normalizeRel, repoAbsolute } from "./path-utils.mjs";
import { buildLiteralRiskScanCommand } from "./literal-risk-command.mjs";
import { buildLiteralRiskOptions } from "./literal-risk-options.mjs";
import {
  compactLiteralRiskReport,
  mapLiteralRiskFindings as mapLiteralRiskFindingsImpl,
} from "./literal-risk-findings.mjs";
import { runJsonProcessToFile } from "./literal-risk-process.mjs";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), ".."));
const BINARY_NAME =
  process.platform === "win32"
    ? "enforcer-literal-scan.exe"
    : "enforcer-literal-scan";

/** Resolves the executable and configuration paths for literal scanning. */
export function resolveLiteralScannerLayout(
  packRoot = PACK_ROOT,
  env = process.env,
) {
  const root = path.join(path.resolve(packRoot), "crates", "enforcer-literal-scan");
  const manifest = path.join(root, "Cargo.toml");
  if (!fs.existsSync(manifest)) {
    throw new Error(`Literal-risk scanner crate is missing: ${manifest}`);
  }
  const configuredTarget = String(env.CARGO_TARGET_DIR ?? "").trim();
  const target = configuredTarget
    ? path.resolve(packRoot, configuredTarget)
    : path.join(path.resolve(packRoot), "target");
  return { root, manifest, target };
}

function builtBinaryCandidates(layout) {
  return [
    path.join(layout.target, "debug", BINARY_NAME),
    path.join(layout.target, "release", BINARY_NAME),
  ];
}

function resolveBuiltBinary(layout) {
  return builtBinaryCandidates(layout).find((candidate) => fs.existsSync(candidate));
}

function newestSourceMtimeMs(dir) {
  let newest = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "target") continue;
      newest = Math.max(newest, newestSourceMtimeMs(fullPath));
      continue;
    }
    if (entry.name.endsWith(".rs") || entry.name === "Cargo.toml") {
      newest = Math.max(newest, fs.statSync(fullPath).mtimeMs);
    }
  }
  return newest;
}

function isBinaryFresh(binaryPath, layout) {
  const binaryMtime = fs.statSync(binaryPath).mtimeMs;
  return binaryMtime >= newestSourceMtimeMs(layout.root);
}

function ensureBuiltBinary() {
  const layout = resolveLiteralScannerLayout();
  const existingBinary = resolveBuiltBinary(layout);
  if (existingBinary && isBinaryFresh(existingBinary, layout)) return existingBinary;
  const build = spawnSync(
    "cargo",
    [
      "build",
      "--quiet",
      "--manifest-path",
      layout.manifest,
      "--bin",
      "enforcer-literal-scan",
    ],
    {
      cwd: PACK_ROOT,
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
  const builtBinary = resolveBuiltBinary(layout);
  if (!builtBinary) {
    throw new Error(
      `literal-risk scanner build completed but produced no binary under ${layout.target}`,
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

/** Runs the literal scanner and normalizes its report for callers. */
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
    .map((file) => normalizeRel(rootPath, repoAbsolute(rootPath, file)))
    .filter(Boolean);
  const command = buildLiteralRiskScanCommand(
    invocation,
    rootPath,
    scannerOptions,
    explicitFiles,
  );
  const result = runJsonProcessToFile(invocation.command, command, {
    cwd: rootPath,
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
    report = compactLiteralRiskReport(JSON.parse(stdout || "{}"));
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

/** Maps literal-scanner output into enforcer findings. */
export function mapLiteralRiskFindings(scan, root, options = {}) {
  return mapLiteralRiskFindingsImpl(scan, root, options);
}
