#!/usr/bin/env node
/*
 * MCP stdio adapter for Ocentra Enforcer.
 * rust-rules-mcp remains a compatibility filename for one release.
 * Split-contract markers:
 * ocentra_enforcer_scan ocentra_enforcer_check
 * decodeScanToolArguments decodeCheckToolArguments decodeCoordinationToolArguments
 * summaryOnly includeScope diagnosticLimit Math.trunc(args.diagnosticLimit)
 * shouldBlockStaleMcpTool COORDINATION_WRITE_TOOLS
 * ocentra_enforcer_mcp_status buildMcpFingerprint
 * ocentra_enforcer_explain runCli("explain"
 * ocentra_enforcer_route buildRouteReport
 * runCli(decoded.cargo ? "cargo" : "scan" read-only scan
 * ocentra_enforcer_coordination_claim ocentra_enforcer_coordination_release
 * function toolError JSON.stringify(body
 */
import fs from "node:fs";
import { PACKAGE_JSON } from "./rust-rules-mcp-helpers.mjs";
import { callTool } from "./rust-rules-mcp-dispatch.mjs";
import { TOOLS } from "./rust-rules-mcp-tool-registry.mjs";
import { startMcpStdioServer } from "./rust-rules-mcp-transport.mjs";

startMcpStdioServer({
  callTool,
  packageJson: PACKAGE_JSON,
  tools: TOOLS,
});
