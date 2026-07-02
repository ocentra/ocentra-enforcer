#!/usr/bin/env node

import {
  buildProofProperties,
} from "./rust-rules-mcp-input-schemas-proof-properties.mjs";

function proofInputSchema(commonInputSchema, scopeSchema, extra = {}) {
  return {
    type: "object",
    additionalProperties: false,
    properties: buildProofProperties(commonInputSchema, scopeSchema, extra),
  };
}

export { proofInputSchema };
