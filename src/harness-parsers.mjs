import path from "node:path";
import { parseJsonLines, parserDiagnostic, rustMessageToDiagnostic } from "./harness-parsers-json-lines.mjs";
import { parseJsonPayload, sarifSeverity } from "./harness-parsers-json-payload.mjs";

function parseDiagnostics({ root, runId, tool, stdout, stderr }) {
  const text = [stdout, stderr].filter(Boolean).join("\n");
  return [
    ...parseJsonLines(root, runId, tool, text),
    ...parseJsonPayload(root, runId, tool, text),
    ...parseTscText(root, runId, tool, text),
    ...parsePytestText(runId, tool, text),
  ];
}

function parseTscText(root, runId, tool, text) {
  const diagnostics = [];
  const re = /^(.+?)\((\d+),(\d+)\):\s+(error|warning)\s+(TS\d+):\s+(.+)$/gmu;
  let match;
  while ((match = re.exec(String(text ?? ""))) !== null) {
    diagnostics.push({
      runId,
      tool,
      language: "typescript",
      severity: match[4],
      ruleId: match[5],
      file: normalizeRel(root, match[1]),
      line: Number(match[2]),
      message: match[6],
      source: null,
    });
  }
  return diagnostics;
}

function parsePytestText(runId, tool, text) {
  const diagnostics = [];
  const re = /^FAILED\s+([^:\s]+(?:::[^\s]+)*)\s+-\s+(.+)$/gmu;
  let match;
  while ((match = re.exec(String(text ?? ""))) !== null) {
    diagnostics.push({
      runId,
      tool,
      language: "python",
      severity: "error",
      ruleId: "pytest",
      file: match[1].split("::")[0],
      line: 1,
      message: match[2],
      source: null,
    });
  }
  return diagnostics;
}

function dedupeDiagnostics(diagnostics) {
  const seen = new Set();
  return diagnostics.filter((diagnostic) => {
    const key = `${diagnostic.tool}|${diagnostic.ruleId}|${diagnostic.file}|${diagnostic.line}|${diagnostic.message}`;
    if (seen.has(key)) return false;
    seen.add(key);
    diagnostic.fingerprint = Buffer.from(key).toString("base64url").slice(0, 24);
    return true;
  });
}

function sortDiagnostics(diagnostics) {
  return [...diagnostics].sort(
    (a, b) =>
      String(a.file ?? "").localeCompare(String(b.file ?? "")) ||
      Number(a.line ?? 0) - Number(b.line ?? 0) ||
      String(a.ruleId ?? "").localeCompare(String(b.ruleId ?? "")) ||
      String(a.message ?? "").localeCompare(String(b.message ?? "")),
  );
}

function dedupeDiagnosticsLegacy(diagnostics) {
  return dedupeDiagnostics(diagnostics);
}

function normalizeRel(root, value) {
  const absolute = path.isAbsolute(value) ? value : path.resolve(root, value);
  const relative = path.relative(root, absolute);
  return relative === "" ? "." : relative.split(path.sep).join("/");
}

export {
  dedupeDiagnosticsLegacy as dedupeDiagnostics,
  parseDiagnostics,
  parseJsonLines,
  parseJsonPayload,
  parsePytestText,
  parseTscText,
  parserDiagnostic,
  rustMessageToDiagnostic,
  sarifSeverity,
  sortDiagnostics,
};
