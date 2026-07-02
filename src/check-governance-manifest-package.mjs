import { uniqueSorted } from "./path-utils.mjs";

export function packageExportTargets(exportsField) {
  const targets = [];

  function visit(value) {
    if (typeof value === "string") {
      if (value.startsWith("./")) targets.push(value.slice(2));
      return;
    }
    if (Array.isArray(value)) {
      for (const entry of value) visit(entry);
      return;
    }
    if (value && typeof value === "object") {
      for (const entry of Object.values(value)) visit(entry);
    }
  }

  visit(exportsField);
  return uniqueSorted(targets);
}
