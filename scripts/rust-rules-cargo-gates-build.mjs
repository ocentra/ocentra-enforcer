import fs from "node:fs";
import path from "node:path";

import { configuredCargoCommand } from "./rust-rules-cargo-command.mjs";

/**
 * Resolve the formatter command for a workspace gate.
 *
 * On Windows, `cargo fmt --all --check` can exceed the process/path limits in
 * a large workspace before rustfmt starts.  This repository's checked-in
 * workspace boundary helper runs the same formatter check in bounded package
 * batches and validates that Cargo did not admit vendor packages.  Use it
 * when present; keep the direct Cargo invocation as the project-independent
 * fallback for repositories that do not ship the helper.
 */
function cargoFmtCommand(root, fmtArgs) {
  const workspaceHelper = path.join(root, "scripts", "check-cargo-workspace-members.mjs");
  if (fmtArgs.includes("--all") && fs.existsSync(workspaceHelper)) {
    return {
      command: process.execPath,
      args: [workspaceHelper, "--fmt-check"],
    };
  }
  return { command: "cargo", args: fmtArgs };
}

/** Runs configured Cargo build and formatting gates for the selected packages. */
export function runCargoBuildGates(root, config, policies, packageArgs, fmtArgs) {
  const violations = [];
  const cargoFmtPolicy = policies[0];
  if (cargoFmtPolicy.enabled) {
    const formatter = cargoFmtCommand(root, fmtArgs);
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoFmt",
        true,
        formatter.command,
        formatter.args,
        "RR-10.1",
      ),
    );
  }

  const cargoClippyPolicy = policies[1];
  if (cargoClippyPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoClippy",
        true,
        "cargo",
        ["clippy", "--locked", ...packageArgs, "--all-targets", "--all-features", "--", "-D", "warnings"],
        "RR-10.2",
      ),
    );
  }

  const cargoTestPolicy = policies[2];
  if (cargoTestPolicy.enabled) {
    const testArgs = ["test", "--locked", ...packageArgs, "--all-features"];
    if (config.cargoTestThreads !== null) {
      testArgs.push("--", `--test-threads=${config.cargoTestThreads}`);
    }
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoTest",
        true,
        "cargo",
        testArgs,
        "RR-10.3",
      ),
    );
  }

  const cargoDocPolicy = policies[3];
  if (cargoDocPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoDoc",
        config.runCargoDoc,
        "cargo",
        ["doc", "--locked", ...packageArgs, "--all-features", "--no-deps"],
        "RR-10.4",
        {
          RUSTDOCFLAGS:
            "-D warnings -D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls -D missing_docs",
        },
      ),
    );
  }
  return violations;
}

export { cargoFmtCommand };
