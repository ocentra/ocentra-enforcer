import path from "node:path";
import { parserDiagnostic } from "./harness-parsers-json-lines.mjs";

export function parseJsonPayload(root, runId, tool, text) {
  const trimmed = String(text ?? "").trim();
  if (!trimmed.startsWith("[") && !trimmed.startsWith("{")) return [];
  try {
    const parsed = JSON.parse(trimmed);
    return diagnosticsForPayload(root, runId, tool, parsed);
  } catch (error) {
    return [parserDiagnostic(runId, tool, error, "malformed JSON payload")];
  }
}

export function sarifSeverity(level) {
  if (level === "error") return "error";
  if (level === "warning") return "warning";
  if (level === "note" || level === "none") return "info";
  return "warning";
}

function diagnosticsForPayload(root, runId, tool, parsed) {
  if (isEslintPayload(parsed)) return eslintDiagnostics(root, runId, tool, parsed);
  if (isBanditPayload(parsed)) return banditDiagnostics(root, runId, tool, parsed);
  if (isPyrightPayload(parsed)) return pyrightDiagnostics(root, runId, tool, parsed.generalDiagnostics);
  if (isSarifPayload(parsed)) return sarifDiagnostics(root, runId, tool, parsed.runs);
  return [];
}

function isEslintPayload(parsed) {
  return Array.isArray(parsed) && parsed.every((entry) => entry.filePath && Array.isArray(entry.messages));
}

function isBanditPayload(parsed) {
  return Array.isArray(parsed) && parsed.every((entry) => entry.filename && entry.code);
}

function isPyrightPayload(parsed) {
  return Array.isArray(parsed?.generalDiagnostics);
}

function isSarifPayload(parsed) {
  return Array.isArray(parsed?.runs);
}

function eslintDiagnostics(root, runId, tool, entries) {
  return entries.flatMap((entry) =>
    entry.messages.map((message) => ({
      runId,
      tool,
      language: "typescript",
      severity: message.severity === 2 ? "error" : "warning",
      ruleId: message.ruleId ?? "eslint",
      file: normalizeRel(root, entry.filePath),
      line: message.line ?? 1,
      message: message.message,
      source: null,
    })),
  );
}

function banditDiagnostics(root, runId, tool, entries) {
  return entries.map((entry) => ({
    runId,
    tool,
    language: "python",
    severity: "error",
    ruleId: entry.code,
    file: normalizeRel(root, entry.filename),
    line: entry.location?.row ?? 1,
    message: entry.message,
    source: null,
  }));
}

function pyrightDiagnostics(root, runId, tool, entries) {
  return entries.map((entry) => ({
    runId,
    tool,
    language: "python",
    severity: entry.severity ?? "error",
    ruleId: "pyright",
    file: entry.file ? normalizeRel(root, entry.file) : ".",
    line: entry.range?.start?.line === undefined ? 1 : entry.range.start.line + 1,
    message: entry.message,
    source: null,
  }));
}

function sarifDiagnostics(root, runId, tool, runs) {
  return runs.flatMap((run) => (run.results ?? []).map((result) => sarifResult(root, runId, tool, result)));
}

function sarifResult(root, runId, tool, result) {
  const location = result.locations?.[0]?.physicalLocation ?? {};
  const region = location.region ?? {};
  const artifactUri = location.artifactLocation?.uri ?? ".";
  return {
    runId,
    tool,
    language: "common",
    severity: sarifSeverity(result.level),
    ruleId: result.ruleId ?? result.rule?.id ?? "sarif",
    file: normalizeRel(root, path.resolve(root, artifactUri)),
    line: region.startLine ?? 1,
    message: result.message?.text ?? result.message?.markdown ?? "SARIF result",
    source: null,
  };
}

function normalizeRel(root, value) {
  const absolute = path.isAbsolute(value) ? value : path.resolve(root, value);
  const relative = path.relative(root, absolute);
  return relative === "" ? "." : relative.split(path.sep).join("/");
}
