import process from "node:process";
import { spawnSync } from "node:child_process";

const CARGO_OUTPUT_MAX_BUFFER = 16 * 1024 * 1024;
const DIAGNOSTIC_MAX_LENGTH = 8000;

export function cargoBuildBatchArgs(batch) {
  return [
    "build",
    "--locked",
    "--package",
    batch.packageName,
    ...batch.selectorArgs,
    "--all-features",
  ];
}

export function runCargoBuildBatch(root, batch) {
  return runCargo(root, cargoBuildBatchArgs(batch));
}

export function runCargoTestBatch(root, batch, testArgs) {
  const args = [
    "test",
    "--locked",
    "--package",
    batch.packageName,
    ...batch.selectorArgs,
    "--all-features",
    ...(testArgs.length > 0 ? ["--", ...testArgs] : []),
  ];
  return runCargo(root, args);
}

function runCargo(root, args) {
  const result = spawnSync("cargo", args, {
    cwd: root,
    env: {
      ...process.env,
      CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? "2",
      CARGO_INCREMENTAL: process.env.CARGO_INCREMENTAL ?? "0",
    },
    encoding: "utf8",
    maxBuffer: CARGO_OUTPUT_MAX_BUFFER,
    shell: false,
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    ...result,
    diagnostic: compactCargoDiagnostic(result),
  };
}

export function compactCargoDiagnostic(result) {
  const output = [result.stdout, result.stderr, result.error?.message]
    .filter(Boolean)
    .join("\n")
    .trim();
  if (output.length <= DIAGNOSTIC_MAX_LENGTH) return output;
  return `... [cargo output truncated; tail preserved] ...\n${output.slice(-DIAGNOSTIC_MAX_LENGTH)}`;
}
