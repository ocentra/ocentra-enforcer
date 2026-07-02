#!/usr/bin/env node

import {
  COMMON_INPUT_SCHEMA,
  SCOPE_SCHEMA,
} from "./rust-rules-mcp-context.mjs";
import {
  coordinationActionInputSchema,
  coordinationInputSchema,
} from "./rust-rules-mcp-input-schemas-coordination.mjs";
import { proofInputSchema as buildProofInputSchema } from "./rust-rules-mcp-input-schemas-proof.mjs";
import { runQueryInputSchema as buildRunQueryInputSchema } from "./rust-rules-mcp-input-schemas-query.mjs";

function runQueryInputSchema() {
  return buildRunQueryInputSchema(COMMON_INPUT_SCHEMA);
}

function proofInputSchema(extra = {}) {
  return buildProofInputSchema(COMMON_INPUT_SCHEMA, SCOPE_SCHEMA, extra);
}

export {
  coordinationActionInputSchema,
  coordinationInputSchema,
  proofInputSchema,
  runQueryInputSchema,
};
