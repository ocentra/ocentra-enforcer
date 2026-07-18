import { readTextSafe } from "./test-doctrine-fs.mjs";
import { CATEGORY_SIGNALS } from "./test-doctrine-signals.mjs";

const CONTENT_SCAN_FILE_CAP = 300;

/** Maps test categories to their report labels. */
export const CATEGORY_LABELS = {
  unit: "Unit tests", integration: "Integration tests", e2e: "End-to-end (Playwright/Cypress)",
  contract: "Contract tests", mutation: "Mutation testing", propertyFuzzing: "Property-based / fuzz testing",
  security: "Security test tooling", snapshot: "Snapshot testing", loadPerformance: "Load/performance testing",
  coverageTooling: "Coverage tooling", concurrencyRaceTests: "Concurrency / race-condition tests",
  idempotencyReplayTests: "Idempotency / replay tests", rollbackCompensationTests: "Rollback / compensation tests",
  timeClockTests: "Time / clock-manipulation tests", economicInvariantTests: "Economic / balance-invariant tests",
  killSwitchTests: "Kill-switch / circuit-breaker tests",
};

function contentEvidence(signal, files, relPaths) {
  if (!signal.content) return [];
  const { filePattern, textPatterns } = signal.content;
  const evidence = [];
  let scanned = 0;
  for (let index = 0; index < relPaths.length && scanned < CONTENT_SCAN_FILE_CAP; index += 1) {
    if (!filePattern.test(relPaths[index])) continue;
    scanned += 1;
    const match = textPatterns.find((pattern) => pattern.test(readTextSafe(files[index])));
    if (match) evidence.push(`${relPaths[index]} (matched: ${match.source})`);
  }
  return evidence;
}

/** Determines the report category for a test surface. */
export function detectCategory(name, relPaths, manifestText, files) {
  const signals = CATEGORY_SIGNALS[name];
  const evidence = [];
  for (const pattern of signals.filenames) {
    const hit = relPaths.find((candidate) => pattern.test(candidate));
    if (hit) evidence.push(hit);
  }
  for (const pattern of signals.manifestText) if (pattern.test(manifestText)) evidence.push(`manifest: ${pattern.source}`);
  evidence.push(...contentEvidence(signals, files, relPaths));
  return { present: evidence.length > 0, evidence: evidence.slice(0, 5) };
}
