#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { createHash } from "node:crypto";
import { SERVER_ROOT } from "./rust-rules-mcp-context.mjs";

function buildMcpFingerprint(mcpFingerprintFiles) {
  const files = mcpFingerprintFiles.map(fingerprintFile);
  const digest = createHash("sha256")
    .update(
      JSON.stringify(
        files.map((file) => ({
          path: file.path,
          exists: file.exists,
          sha256: file.sha256,
          byteLength: file.byteLength,
        })),
      ),
    )
    .digest("hex");
  return {
    digest,
    packageVersion: readPackageVersion(),
    files,
  };
}

function fingerprintFile(filePath) {
  const label = normalizeFingerprintLabel(filePath);
  const resolved = resolveFingerprintFile(filePath);
  if (!fs.existsSync(resolved)) {
    return missingFingerprintEntry(label, resolved);
  }
  const buffer = fs.readFileSync(resolved);
  const stat = fs.statSync(resolved);
  return {
    path: label,
    resolvedPath: resolved,
    exists: true,
    sha256: createHash("sha256").update(buffer).digest("hex"),
    byteLength: buffer.length,
    mtimeMs: stat.mtimeMs,
  };
}

function missingFingerprintEntry(label, resolved) {
  return {
    path: label,
    resolvedPath: resolved,
    exists: false,
    sha256: null,
    byteLength: 0,
    mtimeMs: null,
  };
}

function changedFingerprintFiles(startupFiles, currentFiles) {
  const startupByPath = new Map(startupFiles.map((file) => [file.path, file]));
  const currentByPath = new Map(currentFiles.map((file) => [file.path, file]));
  const paths = [...new Set([...startupByPath.keys(), ...currentByPath.keys()])].sort();
  return paths
    .map((filePath) => {
      const startup = startupByPath.get(filePath);
      const current = currentByPath.get(filePath);
      return fingerprintChange(filePath, startup, current);
    })
    .filter(Boolean);
}

function fingerprintChange(filePath, startup, current) {
  const changed =
    startup?.exists !== current?.exists ||
    startup?.sha256 !== current?.sha256 ||
    startup?.byteLength !== current?.byteLength;
  return changed
    ? {
        path: filePath,
        startup: summarizeFingerprintEntry(startup),
        current: summarizeFingerprintEntry(current),
      }
    : null;
}

function summarizeFingerprintEntry(entry) {
  return entry
    ? {
        exists: entry.exists,
        sha256: entry.sha256,
        byteLength: entry.byteLength,
      }
    : null;
}

function readPackageVersion() {
  try {
    return JSON.parse(
      fs.readFileSync(path.join(SERVER_ROOT, "package.json"), "utf8"),
    ).version;
  } catch {
    return null;
  }
}

function extraFingerprintFiles() {
  return String(process.env.OCENTRA_ENFORCER_MCP_FINGERPRINT_EXTRA ?? "")
    .split(path.delimiter)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function resolveFingerprintFile(filePath) {
  return path.isAbsolute(filePath)
    ? path.resolve(filePath)
    : path.join(SERVER_ROOT, filePath);
}

function normalizeFingerprintLabel(filePath) {
  return path.isAbsolute(filePath)
    ? path.resolve(filePath).replaceAll("\\", "/")
    : filePath.replaceAll("\\", "/");
}

function normalizeToolName(name) {
  return String(name ?? "").replace(/^rust_rules_/u, "ocentra_enforcer_");
}

export {
  buildMcpFingerprint,
  changedFingerprintFiles,
  extraFingerprintFiles,
  fingerprintFile,
  normalizeFingerprintLabel,
  normalizeToolName,
  readPackageVersion,
  resolveFingerprintFile,
  summarizeFingerprintEntry,
};
