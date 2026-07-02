import path from "node:path";

function parseDiagnostics({ root, runId, tool, stdout, stderr }) {
  const text = [stdout, stderr].filter(Boolean).join("\n");
  return [
    ...parseJsonLines(root, runId, tool, text),
    ...parseJsonPayload(root, runId, tool, text),
    ...parseTscText(root, runId, tool, text),
    ...parsePytestText(runId, tool, text),
  ];
}

function parseJsonLines(root, runId, tool, text) {
  const diagnostics = [];
  for (const line of String(text ?? "").split(/\r?\n/u)) {
    if (!line.trim().startsWith("{")) continue;
    try {
      const parsed = JSON.parse(line);
      if (parsed.reason === "compiler-message" && parsed.message) {
        diagnostics.push(rustMessageToDiagnostic(root, runId, tool, parsed.message));
      }
    } catch (error) {
      diagnostics.push(parserDiagnostic(runId, tool, error, "malformed JSON line"));
    }
  }
  return diagnostics;
}

function parseJsonPayload(root, runId, tool, text) {
  const trimmed = String(text ?? "").trim();
  if (!trimmed.startsWith("[") && !trimmed.startsWith("{")) return [];
  try {
    const parsed = JSON.parse(trimmed);
    if (
      Array.isArray(parsed) &&
      parsed.every((entry) => entry.filePath && Array.isArray(entry.messages))
    ) {
      return parsed.flatMap((entry) =>
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
    if (Array.isArray(parsed) && parsed.every((entry) => entry.filename && entry.code)) {
      return parsed.map((entry) => ({
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
    if (Array.isArray(parsed.generalDiagnostics)) {
      return parsed.generalDiagnostics.map((entry) => ({
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
    if (Array.isArray(parsed.runs)) {
      return parsed.runs.flatMap((run) =>
        (run.results ?? []).map((result) => {
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
        }),
      );
    }
  } catch (error) {
    return [parserDiagnostic(runId, tool, error, "malformed JSON payload")];
  }
  return [];
}

function sarifSeverity(level) {
  if (level === "error") return "error";
  if (level === "warning") return "warning";
  if (level === "note" || level === "none") return "info";
  return "warning";
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

function rustMessageToDiagnostic(root, runId, tool, message) {
  const span = message.spans?.find((candidate) => candidate.is_primary) ?? message.spans?.[0] ?? null;
  return {
    runId,
    tool,
    language: "rust",
    severity: message.level === "error" ? "error" : "warning",
    ruleId: message.code?.code ?? "rustc",
    file: span?.file_name ? normalizeRel(root, span.file_name) : ".",
    line: span?.line_start ?? 1,
    message: message.message,
    source: null,
  };
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

function parserDiagnostic(runId, tool, error, context) {
  return {
    runId,
    tool,
    language: inferLanguage(tool),
    severity: "warning",
    ruleId: "HAR-2.4",
    file: ".",
    line: 1,
    message: `Harness parser ignored ${context}: ${error instanceof Error ? error.message : String(error)}`,
    source: null,
  };
}

function inferLanguage(tool) {
  const normalized = String(tool ?? "").toLowerCase();
  if (normalized.includes("cargo") || normalized.includes("rust")) return "rust";
  if (normalized.includes("ts") || normalized.includes("eslint") || normalized.includes("vite")) return "typescript";
  if (normalized.includes("py") || normalized.includes("ruff") || normalized.includes("pytest")) return "python";
  return "common";
}

function normalizeRel(root, value) {
  const absolute = path.isAbsolute(value) ? value : path.resolve(root, value);
  const relative = path.relative(root, absolute);
  return relative === "" ? "." : relative.split(path.sep).join("/");
}

function nullableNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function dedupeDiagnosticsLegacy(diagnostics) {
  return dedupeDiagnostics(diagnostics);
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
