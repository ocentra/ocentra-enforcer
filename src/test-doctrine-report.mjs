/*
 * Combines category-signal detection with project-nature relevance to produce
 * a gap report: what's present (with evidence), what's missing and why it
 * matters for *this* project's nature, and what's optional/opt-in rather than
 * a real gap (mutation testing, snapshot testing).
 */
import { readTextSafe } from "./test-doctrine-fs.mjs";
import { CATEGORY_SIGNALS } from "./test-doctrine-signals.mjs";

const CONTENT_SCAN_FILE_CAP = 300;

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

function contentEvidence(signal, files, relPaths) {
  if (!signal.content) return [];
  const { filePattern, textPatterns } = signal.content;
  const evidence = [];
  let scanned = 0;
  for (let i = 0; i < relPaths.length && scanned < CONTENT_SCAN_FILE_CAP; i += 1) {
    if (!filePattern.test(relPaths[i])) continue;
    scanned += 1;
    const text = readTextSafe(files[i]);
    const match = textPatterns.find((re) => re.test(text));
    if (match) evidence.push(`${relPaths[i]} (matched: ${match.source})`);
  }
  return evidence;
}

function detectCategory(name, relPaths, manifestText, files) {
  const signals = CATEGORY_SIGNALS[name];
  const evidence = [];
  for (const re of signals.filenames) {
    const hit = relPaths.find((p) => re.test(p));
    if (hit) evidence.push(hit);
  }
  for (const re of signals.manifestText) {
    if (re.test(manifestText)) evidence.push(`manifest: ${re.source}`);
  }
  evidence.push(...contentEvidence(signals, files, relPaths));
  return { present: evidence.length > 0, evidence: evidence.slice(0, 5) };
}

function relevance(category, nature) {
  switch (category) {
    case "unit":
    case "security":
    case "coverageTooling":
      return { relevant: true, tier: "core" };
    case "integration":
      return { relevant: nature.isWebApi || nature.hasAsyncWorkers, tier: "core" };
    case "e2e":
      return { relevant: nature.hasFrontendUi, tier: "core" };
    case "contract":
      return {
        relevant: nature.isWebApi || nature.hasMultiServiceBoundary,
        tier: "core",
      };
    case "propertyFuzzing":
      return { relevant: true, tier: "suggested" };
    case "loadPerformance":
      return { relevant: nature.isWebApi || nature.hasAsyncWorkers, tier: "suggested" };
    case "mutation":
    case "snapshot":
      return { relevant: true, tier: "optional" };
    case "concurrencyRaceTests":
      return {
        relevant: nature.isWebApi || nature.hasAsyncWorkers,
        tier: nature.hasMoneyCriticalSurface ? "core" : "suggested",
      };
    case "idempotencyReplayTests":
      return {
        relevant: nature.isWebApi || nature.hasMultiServiceBoundary || nature.hasMoneyCriticalSurface,
        tier: nature.hasMoneyCriticalSurface ? "core" : "suggested",
      };
    case "rollbackCompensationTests":
      return {
        relevant: nature.hasMoneyCriticalSurface || nature.hasAsyncWorkers,
        tier: nature.hasMoneyCriticalSurface ? "core" : "suggested",
      };
    case "timeClockTests":
      return { relevant: true, tier: nature.hasMoneyCriticalSurface ? "suggested" : "optional" };
    case "economicInvariantTests":
      return { relevant: nature.hasMoneyCriticalSurface, tier: "core" };
    case "killSwitchTests":
      return { relevant: nature.hasMoneyCriticalSurface, tier: "suggested" };
    default:
      return { relevant: false, tier: "optional" };
  }
}

function reasonFor(category, nature) {
  const moneyNote = nature.hasMoneyCriticalSurface
    ? ` This project has money/billing-looking files (${nature.moneyCriticalFiles.slice(0, 3).join(", ")}) — treat idempotency/invariant coverage here as higher priority.`
    : "";
  const clientNote = nature.hasMultiServiceBoundary
    ? ` Candidate boundary files: ${nature.multiServiceClientFiles.slice(0, 3).join(", ")}.`
    : "";
  const REASONS = {
    integration: `Project looks like a web API or async worker — integration tests (real DB/app, not mocks) verify the actual request/consumer lifecycle, not just isolated functions.${moneyNote}`,
    e2e: "A frontend UI was detected — no end-to-end suite means UI regressions can only be caught by hand.",
    contract: `A public API or internal service boundary was detected — without contract tests, either side can silently break the other.${clientNote}`,
    security: "Baseline secret-scanning and static-analysis tooling protects against the cheapest, most common class of incidents.",
    coverageTooling: "No coverage measurement tool found — you can't tell what's actually exercised.",
    propertyFuzzing: "No property-based or API-fuzz tooling found — hand-written examples miss edge cases a generator would find for free.",
    loadPerformance: "No load/perf tooling found — capacity and degradation behavior are untested.",
    concurrencyRaceTests: `No tests found exercising parallel/concurrent requests — race conditions (double-processing, lost updates) only show up under real concurrency, never in sequential tests.${moneyNote}`,
    idempotencyReplayTests: `No tests found asserting that repeating a request doesn't repeat its effect — without this, retries and duplicate deliveries are unverified.${moneyNote}`,
    rollbackCompensationTests: "No tests found for rollback/compensation logic — partial-failure recovery paths are exactly where bugs hide because they're rarely exercised.",
    timeClockTests: "No tests found manipulating time/clock (freezegun, synthetic timers) — expiry, cooldown, and scheduling logic is untested against clock skew or boundary timing.",
    economicInvariantTests: `Money-critical files were detected but no tests assert an explicit invariant (balance conservation, no double-charge) — these are the tests that catch a state bug before it costs money.${moneyNote}`,
    killSwitchTests: "Money-critical files were detected but no tests exercise a kill-switch/circuit-breaker/emergency-disable path — an untested emergency control is a control you can't trust when you need it.",
  };
  return REASONS[category] ?? "Not detected.";
}

function ciInfoFor(category, ci) {
  return ci.perCategory[category] ?? { wired: false, blocking: false, evidence: [] };
}

function ciGapReasonFor(category, ciInfo, ciInfoIncludingUntracked) {
  const uncommittedNote = !ciInfo.blocking && ciInfoIncludingUntracked?.blocking
    ? " (It WOULD be gated if currently-uncommitted CI workflow files were merged — don't credit that until they are.)"
    : "";
  if (!ciInfo.wired) {
    return `Detected locally but never invoked anywhere in committed CI — nothing forces it to run before merge.${uncommittedNote}`;
  }
  return `Runs in CI but every matching step is non-blocking (continue-on-error/allow_failure) — a failure here is invisible, not gated.${uncommittedNote}`;
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
      "Heuristic, signal-based (file names, config files, dependency manifests, CI step text) — not a certification. "
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
