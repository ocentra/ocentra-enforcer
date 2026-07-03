import { Schema } from "effect";
import { RuleRegistrySchema } from "./enforcer-schemas-rules.mjs";
import { ProofRegistrySchema } from "./enforcer-schemas-proof.mjs";
import { ConfigSchema } from "./enforcer-schemas-config.mjs";
import {
  RouteRequestSchema,
  ScanToolArgumentsSchema,
  DoctorToolArgumentsSchema,
  ExplainToolArgumentsSchema,
  CheckToolArgumentsSchema,
  InitRequestSchema,
  CodexInstallRequestSchema,
  CodexUninstallRequestSchema,
  CodexDoctorRequestSchema,
  RunToolArgumentsSchema,
  RunQueryArgumentsSchema,
  ProofRouteRequestSchema,
  ProofRunArgumentsSchema,
  ProofQueryArgumentsSchema,
  ProofClaimArgumentsSchema,
  CoordinationToolArgumentsSchema,
} from "./enforcer-schemas-tools.mjs";
import {
  ScanReportSchema,
  CheckReportSchema,
  RouteReportSchema,
  CoordinationHealthReportSchema,
  CoordinationPresenceReportSchema,
  RunReportSchema,
  ProofRunReportSchema,
  ProofClaimReportSchema,
} from "./enforcer-schemas-reports.mjs";

export function decodeRuleRegistry(value) {
  return decodeWithSchema(RuleRegistrySchema, value, "rule registry");
}

export function decodeProofRegistry(value) {
  return decodeWithSchema(ProofRegistrySchema, value, "proof registry");
}

export function decodeEnforcerConfig(value) {
  return decodeWithSchema(ConfigSchema, value, "enforcer config");
}

export function decodeRouteRequest(value) {
  return decodeWithSchema(RouteRequestSchema, value, "route request");
}

export function decodeScanToolArguments(value) {
  return decodeWithSchema(
    ScanToolArgumentsSchema,
    value,
    "scan tool arguments",
  );
}

export function decodeDoctorToolArguments(value) {
  return decodeWithSchema(
    DoctorToolArgumentsSchema,
    value,
    "doctor tool arguments",
  );
}

export function decodeExplainToolArguments(value) {
  return decodeWithSchema(
    ExplainToolArgumentsSchema,
    value,
    "explain tool arguments",
  );
}

export function decodeCheckToolArguments(value) {
  return decodeWithSchema(
    CheckToolArgumentsSchema,
    value,
    "check tool arguments",
  );
}

export function decodeInitRequest(value) {
  return decodeWithSchema(InitRequestSchema, value, "init request");
}

export function decodeCodexInstallRequest(value) {
  return decodeWithSchema(
    CodexInstallRequestSchema,
    value,
    "codex install request",
  );
}

export function decodeCodexUninstallRequest(value) {
  return decodeWithSchema(
    CodexUninstallRequestSchema,
    value,
    "codex uninstall request",
  );
}

export function decodeCodexDoctorRequest(value) {
  return decodeWithSchema(
    CodexDoctorRequestSchema,
    value,
    "codex doctor request",
  );
}

export function decodeScanReport(value) {
  return decodeWithSchema(ScanReportSchema, value, "scan report");
}

export function decodeCheckReport(value) {
  return decodeWithSchema(CheckReportSchema, value, "check report");
}

export function decodeRouteReport(value) {
  return decodeWithSchema(RouteReportSchema, value, "route report");
}

export function decodeRunToolArguments(value) {
  return decodeWithSchema(RunToolArgumentsSchema, value, "run tool arguments");
}

export function decodeRunQueryArguments(value) {
  return decodeWithSchema(
    RunQueryArgumentsSchema,
    value,
    "run query arguments",
  );
}

export function decodeProofRouteRequest(value) {
  return decodeWithSchema(
    ProofRouteRequestSchema,
    value,
    "proof route request",
  );
}

export function decodeProofRunArguments(value) {
  return decodeWithSchema(
    ProofRunArgumentsSchema,
    value,
    "proof run arguments",
  );
}

export function decodeProofQueryArguments(value) {
  return decodeWithSchema(
    ProofQueryArgumentsSchema,
    value,
    "proof query arguments",
  );
}

export function decodeProofClaimArguments(value) {
  return decodeWithSchema(
    ProofClaimArgumentsSchema,
    value,
    "proof claim arguments",
  );
}

export function decodeCoordinationToolArguments(value) {
  return decodeWithSchema(
    CoordinationToolArgumentsSchema,
    value,
    "coordination tool arguments",
  );
}

export function decodeCoordinationHealthReport(value) {
  return decodeWithSchema(
    CoordinationHealthReportSchema,
    value,
    "coordination health report",
  );
}

export function decodeCoordinationPresenceReport(value) {
  return decodeWithSchema(
    CoordinationPresenceReportSchema,
    value,
    "coordination presence report",
  );
}

export function decodeRunReport(value) {
  return decodeWithSchema(RunReportSchema, value, "run report");
}

export function decodeProofRunReport(value) {
  return decodeWithSchema(ProofRunReportSchema, value, "proof run report");
}

export function decodeProofClaimReport(value) {
  return decodeWithSchema(ProofClaimReportSchema, value, "proof claim report");
}

export function decodeWithSchema(schema, value, label) {
  try {
    return Schema.decodeUnknownSync(schema)(value);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} schema validation failed: ${message}`);
  }
}
