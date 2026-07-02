#!/usr/bin/env node

import { compactScope, maybeCompactReport } from "./rust-rules-mcp-compact-core.mjs";
import {
  compactFinding,
  countBy,
  groupFindings,
  uniqueSorted,
} from "./rust-rules-mcp-compact-grouping.mjs";

function parseJson(text) {
  if (!text || !text.trim().startsWith("{")) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

export {
  compactFinding,
  compactScope,
  countBy,
  groupFindings,
  maybeCompactReport,
  parseJson,
  uniqueSorted,
};
