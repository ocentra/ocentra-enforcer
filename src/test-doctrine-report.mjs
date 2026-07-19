/*
 * Combines category-signal detection with project-nature relevance to produce
 * a gap report: what's present (with evidence), what's missing and why it
 * matters for *this* project's nature, and what's optional/opt-in rather than
 * a real gap (mutation testing, snapshot testing).
 */
import { CATEGORY_SIGNALS } from "./test-doctrine-signals.mjs";
import { detectCategory } from "./test-doctrine-report-detection.mjs";
import { relevance } from "./test-doctrine-report-relevance.mjs";
import { ciGapReasonFor, reasonFor } from "./test-doctrine-report-reasons.mjs";

const CATEGORY_LABELS = {
  unit: "Unit tests",
  integration: "Integration tests",
  e2e: "End-to-end (Playwright/Cypress)",
  contract: "Contract tests",
  mutation: "Mutation testing",
  propertyFuzzing: "Property-based / fuzz testing",
  security: "Security test tooling",
  snapshot: "Snapshot testing",
  loadPerformance: "Load/performance testing",
  coverageTooling: "Coverage tooling",
  concurrencyRaceTests: "Concurrency / race-condition tests",
  idempotencyReplayTests: "Idempotency / replay tests",
  rollbackCompensationTests: "Rollback / compensation tests",
  timeClockTests: "Time / clock-manipulation tests",
  economicInvariantTests: "Economic / balance-invariant tests",
  killSwitchTests: "Kill-switch / circuit-breaker tests",
};

function ciInfoFor(category, ci) {
  return ci.perCategory[category] ?? { wired: false, blocking: false, evidence: [] };
}

function buildReport({ root, relPaths, manifestText, nature, ci, files }) {
  const detected = {};
  const missing = [];
  const ciGaps = [];
  for (const category of Object.keys(CATEGORY_SIGNALS)) {
    const result = detectCategory(category, relPaths, manifestText, files);
    const rel = relevance(category, nature);
    const ciInfo = ciInfoFor(category, ci);
    const ciInfoIncludingUntracked = ci.perCategoryIncludingUntracked?.[category] ?? null;
    detected[category] = {
      label: CATEGORY_LABELS[category],
      ...result,
      relevant: rel.relevant,
      ci: ciInfo,
      ciIncludingUntracked: ciInfoIncludingUntracked,
    };
    if (rel.relevant && !result.present) {
      missing.push({
        category,
        label: CATEGORY_LABELS[category],
        tier: rel.tier,
        reason: reasonFor(category, nature),
      });
    } else if (rel.relevant && result.present && ciInfo && ciInfo.blocking !== true) {
      ciGaps.push({
        category,
        label: CATEGORY_LABELS[category],
        reason: ciGapReasonFor(category, ciInfo, ciInfoIncludingUntracked),
        ciEvidence: ciInfo.evidence,
      });
    }
  }
  const tierOrder = { core: 0, suggested: 1, optional: 2 };
  missing.sort((a, b) => tierOrder[a.tier] - tierOrder[b.tier]);
  return {
    root,
    caveat:
      "Heuristic, signal-based (file names, config files, dependency manifests, CI step text) â€” not a certification. "
      + "Evidence should be opened and judged, not trusted at face value; absence of a signal does not always mean absence of the practice.",
    nature,
    ciConfigFilesFound: ci.ciConfigFilesFound,
    hasUntrackedCiFiles: ci.hasUntrackedCiFiles ?? false,
    detected,
    missing,
    ciGaps,
    summary: {
      categoriesRelevant: Object.values(detected).filter((d) => d.relevant).length,
      categoriesPresent: Object.values(detected).filter((d) => d.relevant && d.present).length,
      categoriesMissing: missing.length,
      coreMissing: missing.filter((m) => m.tier === "core").length,
      ciGaps: ciGaps.length,
    },
  };
}

export { buildReport, CATEGORY_LABELS };
