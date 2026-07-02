#!/usr/bin/env node

const PROOF_CAPABILITIES = [
  "ci",
  "local",
  "windows",
  "linux",
  "macos",
  "wsl",
  "android-emulator",
  "android-device",
  "ios-simulator",
  "ios-device",
  "browser",
  "network",
  "cloud",
  "manual-required",
];

const LEGACY_PATHS_PROPERTY = {
  type: "array",
  items: { type: "string" },
  description:
    "Legacy proof artifact files or directories to import or compare. Defaults to test-results, output, and docs/proof.",
};

const INCLUDE_SCRIPTS_PROPERTY = {
  type: "boolean",
  description:
    "For proof inventory only: include bounded script rows. Defaults to false.",
};

const STATUS_PROPERTY = {
  type: "string",
  enum: ["passed", "failed", "manual-required", "unavailable", "waived"],
};

const PROOF_CAPABILITY_PROPERTY = {
  type: "string",
  enum: PROOF_CAPABILITIES,
};

function buildProofProperties(commonInputSchema, scopeSchema, extra = {}) {
  return {
    root: commonInputSchema.properties.root,
    profile: commonInputSchema.properties.profile,
    scope: scopeSchema,
    files: commonInputSchema.properties.files,
    plan: { type: "string" },
    capability: PROOF_CAPABILITY_PROPERTY,
    proofId: { type: "string" },
    proofIds: { type: "array", items: { type: "string" } },
    runId: { type: "string" },
    command: { type: "array", items: { type: "string" } },
    tags: { type: "array", items: { type: "string" } },
    artifact: { type: "string" },
    legacyPaths: LEGACY_PATHS_PROPERTY,
    limit: { type: "number" },
    diagnosticLimit: { type: "number" },
    limitBytes: { type: "number" },
    includeScripts: INCLUDE_SCRIPTS_PROPERTY,
    status: STATUS_PROPERTY,
    pin: { type: "boolean" },
    claimId: { type: "string" },
    prReady: { type: "boolean" },
    allowDirty: { type: "boolean" },
    dryRun: { type: "boolean" },
    ...extra,
  };
}

export {
  INCLUDE_SCRIPTS_PROPERTY,
  LEGACY_PATHS_PROPERTY,
  PROOF_CAPABILITY_PROPERTY,
  STATUS_PROPERTY,
  buildProofProperties,
};
