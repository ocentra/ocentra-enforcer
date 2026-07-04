import fs from "node:fs";
import path from "node:path";
import { parseFileList } from "./rust-rules-scan-core-args-options.mjs";

export function collectFileTokens(tokens, startIndex, explicitFiles) {
  let index = startIndex;
  while (index < tokens.length && !tokens[index].startsWith("-")) {
    explicitFiles.push(...parseFileList(tokens[index]));
    index += 1;
  }
  return index;
}

function resolveManifestPath(manifestPath, root) {
  if (path.isAbsolute(manifestPath)) return manifestPath;
  const cwdPath = path.resolve(process.cwd(), manifestPath);
  if (fs.existsSync(cwdPath)) return cwdPath;
  return path.resolve(root ?? process.cwd(), manifestPath);
}

function parseManifestPayload(payload, manifestPath) {
  const trimmed = payload.trim();
  if (trimmed === "") return [];
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return parseFileList(payload);
  }
  const parsed = JSON.parse(trimmed);
  const files = Array.isArray(parsed) ? parsed : parsed.files;
  if (!Array.isArray(files)) {
    throw new Error(`File manifest ${manifestPath} must be a JSON array or object with a files array.`);
  }
  return files.flatMap((entry) => parseFileList(entry));
}

export function collectFilesFromManifest(manifestPath, root) {
  const resolved = resolveManifestPath(String(manifestPath ?? ""), root);
  const payload = fs.readFileSync(resolved, "utf8");
  return parseManifestPayload(payload, resolved);
}
