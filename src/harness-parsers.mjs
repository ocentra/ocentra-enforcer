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
    ...parseCiText(root, runId, tool, text),
  ];
}

const ANSI_ESCAPE = /\u001b\[[0-9;]*[A-Za-z]/gu;
const GITHUB_TIMESTAMP = /^\ufeff?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z\s+/u;
const DOWNSTREAM_NOISE = [
  /^error: could not compile .+ due to \d+ previous error/u,
  /^warning: build failed, waiting for other jobs to finish/u,
  /^##\[error\]Process completed with exit code \d+/u,
];

function cleanCiLine(line) {
  return String(line ?? "")
    .replace(GITHUB_TIMESTAMP, "")
    .replace(ANSI_ESCAPE, "")
    .trimEnd();
}

function downstreamNoise(line) {
  return DOWNSTREAM_NOISE.some((pattern) => pattern.test(line.trim()));
}

function sourceLocation(lines, start, root) {
  for (let index = start + 1; index < Math.min(lines.length, start + 8); index += 1) {
    const match = lines[index].trim().match(/^-->\s+(.+?):(\d+):(\d+)$/u);
    if (match) {
      return {
        file: normalizeRel(root, match[1]),
        line: Number(match[2]),
        column: Number(match[3]),
      };
    }
  }
  return null;
}

function missingSourceCause(lines, index, root) {
  const match = lines[index]
    .trim()
    .match(/^error: couldn't read `([^`]+)`: (.+)$/u);
  if (!match) return null;
  return {
    category: "missing-source-file",
    severity: "error",
    message: lines[index].trim(),
    file: normalizeRel(root, match[1]),
    referencedFrom: sourceLocation(lines, index, root),
    confidence: "high",
    action:
      "Fix the source/module path. Verify the referenced file and every intermediate directory exist on the CI OS; avoid #[path] values that traverse through a synthetic inline-module directory.",
  };
}

function compilerCause(lines, index, root) {
  const line = lines[index].trim();
  if (downstreamNoise(line) || !/^(?:error(?:\[[A-Z]\d+\])?:|fatal error:)/u.test(line)) {
    return null;
  }
  const location = sourceLocation(lines, index, root);
  return {
    category: "compiler-error",
    severity: "error",
    message: line,
    file: location?.file ?? ".",
    referencedFrom: location,
    confidence: "medium",
    action: "Repair this first compiler diagnostic before rerunning the full CI workflow.",
  };
}

export function triageCiText({ root, tool = "ci", text }) {
  const lines = String(text ?? "").split(/\r?\n/u).map(cleanCiLine);
  let command = null;
  let commandGroup = null;
  let rootCause = null;
  let rootCauseIndex = -1;

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index].trim();
    const group = line.match(/^##\[group\]Run\s+(.+)$/u);
    if (group) {
      command = group[1];
      commandGroup = line;
    }
    const candidate = missingSourceCause(lines, index, root) ?? compilerCause(lines, index, root);
    if (candidate) {
      rootCause = candidate;
      rootCauseIndex = index;
      break;
    }
  }

  const noise = rootCauseIndex < 0
    ? []
    : lines
        .slice(rootCauseIndex + 1)
        .map((line) => line.trim())
        .filter((line) => downstreamNoise(line));
  return {
    ok: true,
    found: rootCause !== null,
    tool,
    command,
    commandGroup,
    rootCause,
    downstreamNoise: noise,
    reproduction: command,
  };
}

function parseCiText(root, runId, tool, text) {
  const report = triageCiText({ root, tool, text });
  if (!report.rootCause) return [];
  return [{
    runId,
    tool,
    language: "rust",
    severity: report.rootCause.severity,
    ruleId: `HAR-CI-${report.rootCause.category}`,
    file: report.rootCause.referencedFrom?.file ?? report.rootCause.file,
    line: report.rootCause.referencedFrom?.line ?? 1,
    message: report.rootCause.message,
    source: report.command,
  }];
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
  parseCiText,
  parserDiagnostic,
  rustMessageToDiagnostic,
  sarifSeverity,
  sortDiagnostics,
};
