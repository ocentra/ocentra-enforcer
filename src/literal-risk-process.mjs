import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

/** Runs a command that writes a JSON report to the requested file. */
export function runJsonProcessToFile(command, args, options = {}) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-literal-risk-"));
  const stdoutPath = path.join(tempDir, "report.json");
  const stdoutFd = fs.openSync(stdoutPath, "w");
  try {
    const result = spawnSync(command, args, {
      cwd: options.cwd,
      encoding: "utf8",
      shell: false,
      stdio: ["ignore", stdoutFd, "pipe"],
      maxBuffer: options.stderrMaxBuffer ?? 8 * 1024 * 1024,
    });
    fs.closeSync(stdoutFd);
    const stdout = fs.readFileSync(stdoutPath, "utf8");
    return { ...result, stdout, stderr: result.stderr ?? "" };
  } finally {
    try {
      fs.closeSync(stdoutFd);
    } catch {
      // The descriptor was already closed after the child exited.
    }
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}
