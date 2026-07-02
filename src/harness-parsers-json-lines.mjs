import path from "node:path";

export function parseJsonLines(root, runId, tool, text) {
  const diagnostics = [];
  for (const line of String(text ?? "").split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("{")) continue;
    diagnostics.push(...parseJsonLine(root, runId, tool, trimmed));
  }
  return diagnostics;
}

export function rustMessageToDiagnostic(root, runId, tool, message) {
  const span = primarySpan(message);
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

export function parserDiagnostic(runId, tool, error, context) {
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

function parseJsonLine(root, runId, tool, line) {
  try {
    const parsed = JSON.parse(line);
    return maybeRustCompilerMessage(root, runId, tool, parsed);
  } catch (error) {
    return [parserDiagnostic(runId, tool, error, "malformed JSON line")];
  }
}

function maybeRustCompilerMessage(root, runId, tool, parsed) {
  if (parsed.reason === "compiler-message" && parsed.message) {
    return [rustMessageToDiagnostic(root, runId, tool, parsed.message)];
  }
  return [];
}

function primarySpan(message) {
  return message.spans?.find((candidate) => candidate.is_primary) ?? message.spans?.[0] ?? null;
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
