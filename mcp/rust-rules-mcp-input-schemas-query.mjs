#!/usr/bin/env node

function runQueryInputSchema(commonInputSchema) {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      root: commonInputSchema.properties.root,
      runId: {
        type: "string",
        description: "Optional run id. Defaults to the latest run.",
      },
      limit: {
        type: "number",
        description: "Maximum run or diagnostic rows to return.",
      },
      diagnosticLimit: {
        type: "number",
        description: "Maximum diagnostics for last-failure.",
      },
      severity: { type: "string", enum: ["error", "warning", "info"] },
      status: {
        type: "string",
        enum: ["passed", "failed"],
        description: "Optional run status filter.",
      },
      file: {
        type: "string",
        description: "Optional file filter for diagnostics.",
      },
      tool: { type: "string", description: "Optional logical tool filter." },
      crateName: {
        type: "string",
        description: "Optional Cargo crate/package metadata filter.",
      },
      packageName: {
        type: "string",
        description: "Optional JS/Python package metadata filter.",
      },
      domain: {
        type: "string",
        description: "Optional domain metadata filter.",
      },
      tag: { type: "string", description: "Optional run tag filter." },
      artifact: {
        type: "string",
        enum: ["stdout", "stderr", "diagnostics", "events"],
      },
      limitBytes: {
        type: "number",
        description: "Maximum artifact bytes to return.",
      },
    },
  };
}

export { runQueryInputSchema };
