import path from "node:path";
import { parserDiagnostic } from "./harness-parsers-json-lines.mjs";
import {
  banditDiagnostics,
  eslintDiagnostics,
  pyrightDiagnostics,
  sarifDiagnostics,
} from "./harness-parsers-json-diagnostics.mjs";

const PAYLOAD_HANDLERS = [
  [isEslintPayload, eslintDiagnostics],
  [isBanditPayload, banditDiagnostics],
  [isPyrightPayload, pyrightPayloadDiagnostics],
  [isSarifPayload, sarifPayloadDiagnostics],
];

export function parseJsonPayload(root, runId, tool, text) {
  const trimmed = String(text ?? "").trim();
  if (!looksLikeJson(trimmed)) return [];
  const parsed = parsePayload(runId, tool, trimmed);
  return parsed.ok
    ? diagnosticsForPayload(root, runId, tool, parsed.value)
    : [parsed.diagnostic];
}

export function sarifSeverity(level) {
  if (level === "error") return "error";
  if (level === "warning") return "warning";
  if (level === "note" || level === "none") return "info";
  return "warning";
}

function diagnosticsForPayload(root, runId, tool, parsed) {
  for (const [predicate, handler] of PAYLOAD_HANDLERS) {
    if (predicate(parsed)) return handler(root, runId, tool, parsed);
  }
  return [];
}

function looksLikeJson(trimmed) {
  return trimmed.startsWith("[") || trimmed.startsWith("{");
}

function parsePayload(runId, tool, trimmed) {
  try {
    return { ok: true, value: JSON.parse(trimmed) };
  } catch (error) {
    return {
      ok: false,
      diagnostic: parserDiagnostic(runId, tool, error, "malformed JSON payload"),
    };
  }
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

function pyrightPayloadDiagnostics(root, runId, tool, parsed) {
  return pyrightDiagnostics(root, runId, tool, parsed.generalDiagnostics);
}

function sarifPayloadDiagnostics(root, runId, tool, parsed) {
  return sarifDiagnostics(root, runId, tool, parsed.runs, sarifSeverity);
}

function normalizeRel(root, value) {
  const absolute = path.isAbsolute(value) ? value : path.resolve(root, value);
  const relative = path.relative(root, absolute);
  return relative === "" ? "." : relative.split(path.sep).join("/");
}
