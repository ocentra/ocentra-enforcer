import {
  COMMON_INPUT_SCHEMA,
  COMPACT_RESULT_SCHEMA,
} from "./rust-rules-mcp-helpers.mjs";

const EXPLAIN_INPUT_SCHEMA = {
  type: "object",
  additionalProperties: false,
  required: ["ruleId"],
  properties: {
    ruleId: {
      type: "string",
      description: "Rule ID such as RR-7.3.",
    },
  },
};

export const RULE_TOOLS = [
  {
    name: "ocentra_enforcer_mcp_status",
    description:
      "Report MCP process freshness and whether Codex must reload this Enforcer server before coordination writes.",
    inputSchema: {
      type: "object",
      additionalProperties: false,
      properties: {},
    },
  },
  {
    name: "ocentra_enforcer_route",
    description:
      "Return compact indexed Ocentra Enforcer rule docs relevant to files, crate, scope, profile, or one rule ID.",
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        ruleId: {
          type: "string",
          description:
            "Optional explicit rule ID such as RR-7.3. When provided, routes directly to that rule.",
        },
      },
    },
  },
  {
    name: "ocentra_enforcer_scan",
    description:
      "Run deterministic Ocentra Enforcer scanner by workspace, files, crate, or diff scope.",
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        cargo: {
          type: "boolean",
          description:
            "When true, run cargo gates in addition to scanner checks.",
          default: false,
        },
        ...COMPACT_RESULT_SCHEMA,
      },
    },
  },
  {
    name: "ocentra_enforcer_check",
    description:
      "Run a named Ocentra Enforcer reusable check such as no-zod-source, source-shape, dependency-policy, or sbom.",
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      required: ["check"],
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        check: {
          type: "string",
          enum: [
            "no-zod-source",
            "no-naked-domain-strings",
            "no-test-doubles",
            "weak-assertions",
            "skipped-focused-tests",
            "validation-bypass",
            "placeholder-implementation",
            "reexports",
            "cross-platform-script-commands",
            "generated-artifacts",
            "secrets",
            "rust-string-boundaries",
            "source-shape",
            "required-tests",
            "single-source-contracts",
            "dependency-policy",
            "sbom",
            "ai-rule-index",
            "import-boundaries",
            "architecture-policy",
          ],
          description: "Named reusable check to run.",
        },
        checkConfigPath: {
          type: "string",
          description:
            "Optional check-specific config path, for example a single-source contract config.",
        },
        output: {
          type: "string",
          description: "Optional output directory for checks such as sbom.",
        },
        dryRun: {
          type: "boolean",
          description:
            "Validate the check path without writing generated outputs where supported.",
        },
        staged: {
          type: "boolean",
          description: "With check secrets: scan staged files only.",
        },
        tracked: {
          type: "boolean",
          description:
            "With check generated-artifacts: include tracked generated paths.",
        },
        strictEmptyTestTrees: {
          type: "boolean",
          description:
            "With check required-tests: reject tests/proof trees that only contain .gitkeep.",
        },
        ...COMPACT_RESULT_SCHEMA,
      },
    },
  },
  {
    name: "ocentra_enforcer_doctor",
    description:
      "Check Ocentra Enforcer wiring for a target root/config/scope without changing files.",
    inputSchema: COMMON_INPUT_SCHEMA,
  },
  {
    name: "ocentra_enforcer_explain",
    description:
      "Explain one Ocentra Enforcer rule ID and give the docs anchor/fix hint.",
    inputSchema: EXPLAIN_INPUT_SCHEMA,
  },
];
