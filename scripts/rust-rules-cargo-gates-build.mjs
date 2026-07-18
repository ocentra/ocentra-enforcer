import { configuredCargoCommand } from "./rust-rules-cargo-command.mjs";

/** Runs configured Cargo build and formatting gates for the selected packages. */
export function runCargoBuildGates(root, config, policies, packageArgs, fmtArgs) {
  const violations = [];
  const cargoFmtPolicy = policies[0];
  if (cargoFmtPolicy.enabled) {
    violations.push(
      ...configuredCargoCommand(
        root,
        config,
        "cargoFmt",
        true,
        "cargo",
        fmtArgs,
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
