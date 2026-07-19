import { readTextSafe } from "./test-doctrine-fs.mjs";
import { CATEGORY_SIGNALS } from "./test-doctrine-signals.mjs";

const CONTENT_SCAN_FILE_CAP = 300;

function contentEvidence(signal, files, relPaths) {
  if (!signal.content) return [];
  const { filePattern, textPatterns } = signal.content;
  const evidence = [];
  let scanned = 0;
  for (
    let index = 0;
    index < relPaths.length && scanned < CONTENT_SCAN_FILE_CAP;
    index += 1
  ) {
    if (!filePattern.test(relPaths[index])) continue;
    scanned += 1;
    const text = readTextSafe(files[index]);
    const match = textPatterns.find((pattern) => pattern.test(text));
    if (match) {
      evidence.push(`${relPaths[index]} (matched: ${match.source})`);
    }
  }
  return evidence;
}

/** Detects one test category and returns bounded evidence. */
export function detectCategory(name, relPaths, manifestText, files) {
  const signals = CATEGORY_SIGNALS[name];
  const evidence = [];
  for (const pattern of signals.filenames) {
    const hit = relPaths.find((candidate) => pattern.test(candidate));
    if (hit) evidence.push(hit);
  }
  for (const pattern of signals.manifestText) {
    if (pattern.test(manifestText)) {
      evidence.push(`manifest: ${pattern.source}`);
    }
  }
  evidence.push(...contentEvidence(signals, files, relPaths));
  return { present: evidence.length > 0, evidence: evidence.slice(0, 5) };
}
