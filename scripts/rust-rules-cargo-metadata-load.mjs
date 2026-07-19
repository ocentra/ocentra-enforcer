import { spawnSync } from "node:child_process";

const CARGO_METADATA_MAX_BUFFER = 32 * 1024 * 1024;

function runCargoMetadata(root, args) {
  return spawnSync(
    "cargo",
    ["metadata", "--locked", "--format-version", "1", ...args],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: CARGO_METADATA_MAX_BUFFER,
      shell: false,
    },
  );
}

function decodeCargoMetadataResult(result) {
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
  if (result.error) {
    return {
      metadata: null,
      output: output || result.error.message,
      unavailable: result.error.code === "ENOENT",
    };
  }
  if ((result.status ?? 1) !== 0) {
    return { metadata: null, output, unavailable: false };
  }
  return { metadata: JSON.parse(result.stdout), output: "", unavailable: false };
}

/** Identifies Cargo's deterministic stale-lock refusal. */
export function cargoLockNeedsUpdate(output) {
  return /(?:lock file|Cargo\.lock)[^\n]*needs to be updated[^\n]*--locked/iu.test(output);
}

/** Loads metadata without allowing Cargo.lock mutation or unnecessary resolution. */
export function loadLockedCargoMetadata(root) {
  // Resolve the complete locked workspace first: a stale lock must be a hard
  // policy failure. After that proof, inspect direct packages only.
  const locked = decodeCargoMetadataResult(runCargoMetadata(root, ["--offline"]));
  if (!locked.metadata || locked.unavailable) return locked;

  const direct = decodeCargoMetadataResult(
    runCargoMetadata(root, ["--no-deps", "--offline"]),
  );
  if (direct.metadata || direct.unavailable || cargoLockNeedsUpdate(direct.output)) {
    return direct;
  }
  return decodeCargoMetadataResult(runCargoMetadata(root, ["--no-deps"]));
}
