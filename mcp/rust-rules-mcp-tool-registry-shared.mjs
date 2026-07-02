import { COMMON_INPUT_SCHEMA } from "./rust-rules-mcp-helpers.mjs";

const LANGUAGE_ENUM = ["rust", "typescript", "python", "common"];

const OPTIONAL_METADATA_PROPERTIES = {
  cwd: stringProperty("Optional working directory relative to root."),
  runId: stringProperty("Optional caller-provided run id."),
  crateName: stringProperty("Optional Cargo crate/package metadata."),
  packageName: stringProperty("Optional JS/Python package metadata."),
  domain: stringProperty("Optional domain metadata."),
};

export function harnessRunInputSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: ["command"],
    properties: {
      root: COMMON_INPUT_SCHEMA.properties.root,
      profile: COMMON_INPUT_SCHEMA.properties.profile,
      tool: stringProperty(
        "Logical tool name such as cargo-check, eslint, pytest, or tsc.",
      ),
      language: {
        type: "string",
        enum: LANGUAGE_ENUM,
      },
      ...OPTIONAL_METADATA_PROPERTIES,
      command: stringArrayProperty("Executable and arguments."),
      tags: stringArrayProperty(),
    },
  };
}

function stringProperty(description) {
  return { type: "string", description };
}

function stringArrayProperty(description = undefined) {
  return {
    type: "array",
    items: { type: "string" },
    ...(description ? { description } : {}),
  };
}

