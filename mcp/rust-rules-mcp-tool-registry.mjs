import {
  COORDINATION_TOOLS,
} from "./rust-rules-mcp-tool-registry-coordination.mjs";
import { PROOF_TOOLS } from "./rust-rules-mcp-tool-registry-proof.mjs";
import {
  HARNESS_TOOLS,
} from "./rust-rules-mcp-tool-registry-harness.mjs";
import { RULE_TOOLS } from "./rust-rules-mcp-tool-registry-rules.mjs";

const CANONICAL_TOOLS = [
  ...RULE_TOOLS,
  ...HARNESS_TOOLS,
  ...PROOF_TOOLS,
  ...COORDINATION_TOOLS,
];

const LEGACY_ALIAS_TOOLS = CANONICAL_TOOLS.map((tool) => ({
  ...tool,
  name: tool.name.replace("ocentra_enforcer_", "rust_rules_"),
  description: `Legacy alias for ${tool.name}; kept for one Rust-pack compatibility release.`,
}));

export const TOOLS = [...CANONICAL_TOOLS, ...LEGACY_ALIAS_TOOLS];

export const TOOL_SCHEMAS = new Map(
  TOOLS.map((tool) => [tool.name, tool.inputSchema]),
);
