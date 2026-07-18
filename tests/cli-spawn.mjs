import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const TEST_CLI_MAX_BUFFER = 32 * 1024 * 1024;

export function spawnCli(command, args, options = {}) {
  const captureDir = fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-cli-"));
  const stdoutPath = path.join(captureDir, "stdout.log");
  const stderrPath = path.join(captureDir, "stderr.log");
  const stdoutFd = fs.openSync(stdoutPath, "w");
  const stderrFd = fs.openSync(stderrPath, "w");
  let result;
  try {
    result = spawnSync(command, args, {
      ...options,
      maxBuffer: options.maxBuffer ?? TEST_CLI_MAX_BUFFER,
      stdio: ["ignore", stdoutFd, stderrFd],
    });
  } finally {
    fs.closeSync(stdoutFd);
    fs.closeSync(stderrFd);
  }

  const stdout = fs.readFileSync(stdoutPath, "utf8");
  const capturedStderr = fs.readFileSync(stderrPath, "utf8");
  const spawnDiagnostic = result.error?.code === "ETIMEDOUT"
    ? `spawnCli timed out after ${options.timeout}ms: ${path.basename(command)} ${args.slice(0, 12).join(" ")}`
    : result.error
      ? `spawnCli failed (${result.error.code ?? "unknown"}): ${result.error.message}`
      : "";
  const stderr = [capturedStderr.trimEnd(), spawnDiagnostic]
    .filter(Boolean)
    .join("\n");
  return {
    ...result,
    stdout,
    stderr,
    output: [null, stdout, stderr],
  };
}
