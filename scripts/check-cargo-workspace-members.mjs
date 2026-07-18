#!/usr/bin/env node

import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { cargoFmtBatches } from "./check-cargo-workspace-members-format.mjs";
import { validateCargoWorkspaceMembers } from "./check-cargo-workspace-members-validation.mjs";

function cargoFmtCheck(root, workspacePackages) {
  for (const packageNames of cargoFmtBatches(workspacePackages)) {
    const args = ["fmt", "--check", ...packageNames.flatMap((name) => ["-p", name])];
    const result = spawnSync("cargo", args, {
      cwd: root,
      encoding: "utf8",
      shell: process.platform === "win32",
      stdio: "inherit",
    });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}

function cargoMetadata(root) {
  const result = spawnSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    cwd: root,
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  if (result.status !== 0) {
    throw new Error(`cargo metadata failed:\n${result.stderr || result.stdout}`);
  }
  return JSON.parse(result.stdout);
}

function main() {
  const root = process.cwd();
  const result = validateCargoWorkspaceMembers(root, cargoMetadata(root));
  if (!result.ok) {
    for (const error of result.errors) console.error(`ERROR: ${error}`);
    process.exitCode = 1;
    return;
  }
  console.log(`Cargo workspace boundary OK: ${result.workspace.length} product packages, 0 vendor packages.`);
  if (process.argv.includes("--fmt-check")) {
    cargoFmtCheck(root, result.workspacePackages);
    console.log(`Cargo fmt boundary OK: ${result.workspacePackages.length} product packages, 0 vendor packages.`);
  }
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) main();
