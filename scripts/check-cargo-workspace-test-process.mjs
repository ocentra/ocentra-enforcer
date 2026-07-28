import process from "node:process";
import { spawnSync } from "node:child_process";

const CARGO_OUTPUT_MAX_BUFFER = 16 * 1024 * 1024;
const DIAGNOSTIC_STREAM_MAX_LENGTH = 3600;

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
  const status = [
    `cargo status: ${result.status ?? "unknown"}`,
    result.signal ? `signal: ${result.signal}` : null,
    result.error?.message ? `spawn error: ${result.error.message}` : null,
  ]
    .filter(Boolean)
    .join("; ");
  const sections = [
    compactDiagnosticStream("stdout", result.stdout),
    compactDiagnosticStream("stderr", result.stderr),
  ].filter(Boolean);
  return [status, ...sections].join("\n");
}

function compactDiagnosticStream(label, value) {
  const output = value?.trim();
  if (!output) return "";
  if (output.length <= DIAGNOSTIC_STREAM_MAX_LENGTH) {
    return `${label}:\n${output}`;
  }
  return `${label}:\n... [${label} truncated; tail preserved] ...\n${output.slice(
    -DIAGNOSTIC_STREAM_MAX_LENGTH,
  )}`;
}
