import path from "node:path";

export function eslintDiagnostics(root, runId, tool, entries) {
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

export function banditDiagnostics(root, runId, tool, entries) {
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

export function pyrightDiagnostics(root, runId, tool, entries) {
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

export function sarifDiagnostics(root, runId, tool, runs, sarifSeverity) {
  return runs.flatMap((run) =>
    (run.results ?? []).map((result) =>
      sarifResult(root, runId, tool, result, sarifSeverity),
    ),
  );
}

function sarifResult(root, runId, tool, result, sarifSeverity) {
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
