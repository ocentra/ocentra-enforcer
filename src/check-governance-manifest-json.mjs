import fs from "node:fs";
import { escapeRegExp } from "../scripts/check-source-core-helpers.mjs";
import { manifestJsonParseFailure } from "./check-governance-manifest-errors.mjs";

export function parsePackageManifest(packageJsonPath) {
  try {
    return { ok: true, manifest: JSON.parse(fs.readFileSync(packageJsonPath, "utf8")) };
  } catch (error) {
    return manifestJsonParseFailure(packageJsonPath, error);
  }
}

export function lineForJsonKey(filePath, key) {
  if (!fs.existsSync(filePath)) return 1;
  const pattern = new RegExp(`"${escapeRegExp(key)}"\\s*:`, "u");
  return (
    fs
      .readFileSync(filePath, "utf8")
      .split(/\r?\n/u)
      .findIndex((line) => pattern.test(line)) + 1 || 1
  );
}
