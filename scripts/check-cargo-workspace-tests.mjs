#!/usr/bin/env node

import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { parseWorkspaceTestArgs } from "./check-cargo-workspace-test-cli.mjs";
import {
  cleanupTargetArtifacts,
  workspaceTestBatches,
  workspaceTestPlan,
} from "./check-cargo-workspace-test-plan.mjs";
import {
  runCargoBuildBatch,
  runCargoTestBatch,
} from "./check-cargo-workspace-test-process.mjs";

const CARGO_METADATA_MAX_BUFFER = 32 * 1024 * 1024;

/** Runs every Cargo workspace target in bounded batches without accumulating binaries. */
export function runWorkspaceTests(root, packageFilter = null, testArgs = []) {
  const metadata = cargoMetadata(root);
  const packages = metadata.packages
    .filter((pkg) => packageFilter === null || packageFilter.includes(pkg.name))
    .sort((left, right) => left.name.localeCompare(right.name));
  if (packages.length === 0) {
    throw new Error("Cargo workspace test plan selected no packages.");
  }

  const plan = workspaceTestPlan(packages);
  const batches = workspaceTestBatches(plan);
  const binaryEntries = plan.filter((entry) => entry.kind === "bin");
  const binaryBatches = workspaceTestBatches(binaryEntries);
  console.log(
    `Cargo bounded test plan: ${plan.length} target(s) in ${batches.length} batch(es).`,
  );
  try {
    for (const batch of binaryBatches) {
      console.log(
        `\n==> cargo build -p ${batch.packageName} ${batch.selector}`,
      );
      const result = runCargoBuildBatch(root, batch);
      if (result.status !== 0) {
        console.error(
          result.diagnostic || "cargo build process returned no diagnostic.",
        );
        return result.status ?? 1;
      }
    }
    for (const batch of batches) {
      console.log(`\n==> cargo test -p ${batch.packageName} ${batch.selector}`);
      const result = runCargoTestBatch(root, batch, testArgs);
      for (const entry of batch.entries.filter(
        (entry) => entry.kind !== "bin",
      )) {
        cleanupTargetArtifacts(metadata.target_directory, entry);
      }
      if (result.status !== 0) {
        console.error(
          result.diagnostic || "cargo test process returned no diagnostic.",
        );
        return result.status ?? 1;
      }
    }
    console.log(
      `\nCargo bounded workspace tests passed: ${plan.length} target(s).`,
    );
    return 0;
  } finally {
    for (const entry of binaryEntries) {
      cleanupTargetArtifacts(metadata.target_directory, entry);
    }
  }
}

function cargoMetadata(root) {
  const result = spawnSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1"],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: CARGO_METADATA_MAX_BUFFER,
      shell: false,
    },
  );
  if (result.status !== 0) {
    throw new Error(
      `cargo metadata failed:\n${result.stderr || result.stdout}`,
    );
  }
  return JSON.parse(result.stdout);
}

export { cleanupTargetArtifacts, workspaceTestBatches, workspaceTestPlan };

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    const { packageFilter, testArgs } = parseWorkspaceTestArgs(
      process.argv.slice(2),
    );
    process.exitCode = runWorkspaceTests(
      process.cwd(),
      packageFilter,
      testArgs,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
