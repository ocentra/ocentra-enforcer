#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SERVER_ROOT = path.resolve(
  path.join(path.dirname(fileURLToPath(import.meta.url)), ".."),
);
const CLI_PATH = path.join(SERVER_ROOT, "scripts", "rust-rules.mjs");
const RULE_REGISTRY_PATH = path.join(SERVER_ROOT, "rules", "rules.json");
const PACKAGE_JSON = JSON.parse(
  fs.readFileSync(path.join(SERVER_ROOT, "package.json"), "utf8"),
);
const SERVER_STARTED_AT = new Date().toISOString();
const MCP_PROTOCOL_VERSION = "2025-06-18";

const COORDINATION_WRITE_TOOLS = new Set([
  "ocentra_enforcer_coordination_init",
  "ocentra_enforcer_coordination_claim",
  "ocentra_enforcer_coordination_closeout",
  "ocentra_enforcer_coordination_release",
  "ocentra_enforcer_coordination_report",
  "ocentra_enforcer_coordination_message",
  "ocentra_enforcer_coordination_sync",
  "ocentra_enforcer_coordination_ensure",
  "ocentra_enforcer_coordination_compact",
]);

const SCOPE_SCHEMA = {
  type: "string",
  enum: ["workspace", "files", "crate", "diff"],
  description:
    "Validation scope. Defaults to workspace unless files, crateName, or base/head imply a narrower scope.",
};

const COMMON_INPUT_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    root: {
      type: "string",
      description:
        "Target repository root. Defaults to the MCP server working directory.",
    },
    configPath: {
      type: "string",
      description:
        "Optional Ocentra Enforcer config path. Relative paths resolve against root.",
    },
    profile: {
      type: "string",
      description:
        "Optional named pack profile such as strict or ocentra-parent. Ignored when configPath is provided.",
    },
    scope: SCOPE_SCHEMA,
    files: {
      type: "array",
      items: { type: "string" },
      description: "Files or directories for files scope.",
    },
    crateName: {
      type: "string",
      description: "Cargo package name for crate scope.",
    },
    languages: {
      type: "array",
      items: {
        type: "string",
        enum: ["rust", "typescript", "python", "common"],
      },
      description:
        "Optional scan languages. Defaults to the target config/profile.",
    },
    base: {
      type: "string",
      description: "Base git ref for diff scope.",
    },
    head: {
      type: "string",
      description: "Head git ref for diff scope.",
    },
  },
};

const COMPACT_RESULT_SCHEMA = {
  diagnosticLimit: {
    type: "number",
    description: "Maximum findings to include in compact MCP output.",
  },
  summaryOnly: {
    type: "boolean",
    description:
      "Return only summary/group counts without individual findings.",
  },
  groupBy: {
    type: "string",
    enum: ["file", "slice"],
    description: "Group compact findings by file or top-level repo slice.",
  },
  includeScope: {
    type: "boolean",
    description: "When false, omit full scope file lists from MCP output.",
  },
};

export {
  CLI_PATH,
  COMMON_INPUT_SCHEMA,
  COMPACT_RESULT_SCHEMA,
  COORDINATION_WRITE_TOOLS,
  MCP_PROTOCOL_VERSION,
  PACKAGE_JSON,
  RULE_REGISTRY_PATH,
  SERVER_ROOT,
  SERVER_STARTED_AT,
  SCOPE_SCHEMA,
};
