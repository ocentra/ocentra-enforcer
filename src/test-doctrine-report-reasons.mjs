/** Produces the relevance reason for a test category and nature. */
export function reasonFor(category, nature) {
  const moneyNote = nature.hasMoneyCriticalSurface
    ? ` This project has money/billing-looking files (${nature.moneyCriticalFiles.slice(0, 3).join(", ")}); treat idempotency/invariant coverage here as higher priority.` : "";
  const clientNote = nature.hasMultiServiceBoundary
    ? ` Candidate boundary files: ${nature.multiServiceClientFiles.slice(0, 3).join(", ")}.` : "";
  const reasons = {
    integration: `Project looks like a web API or async worker; integration tests (real DB/app, not mocks) verify the actual request/consumer lifecycle, not just isolated functions.${moneyNote}`,
    e2e: "A frontend UI was detected; no end-to-end suite means UI regressions can only be caught by hand.",
    contract: `A public API or internal service boundary was detected; without contract tests, either side can silently break the other.${clientNote}`,
    security: "Baseline secret-scanning and static-analysis tooling protects against the cheapest, most common class of incidents.",
    coverageTooling: "No coverage measurement tool found; you cannot tell what is actually exercised.",
    propertyFuzzing: "No property-based or API-fuzz tooling found; hand-written examples miss edge cases a generator would find for free.",
    loadPerformance: "No load/perf tooling found; capacity and degradation behavior are untested.",
    concurrencyRaceTests: `No tests found exercising parallel/concurrent requests; race conditions (double-processing, lost updates) only show up under real concurrency, never in sequential tests.${moneyNote}`,
    idempotencyReplayTests: `No tests found asserting that repeating a request does not repeat its effect; without this, retries and duplicate deliveries are unverified.${moneyNote}`,
    rollbackCompensationTests: "No tests found for rollback/compensation logic; partial-failure recovery paths are exactly where bugs hide because they are rarely exercised.",
    timeClockTests: "No tests found manipulating time/clock (freezegun, synthetic timers); expiry, cooldown, and scheduling logic is untested against clock skew or boundary timing.",
    economicInvariantTests: `Money-critical files were detected but no tests assert an explicit invariant (balance conservation, no double-charge); these are the tests that catch a state bug before it costs money.${moneyNote}`,
    killSwitchTests: "Money-critical files were detected but no tests exercise a kill-switch/circuit-breaker/emergency-disable path; an untested emergency control is a control you cannot trust when you need it.",
  };
  return reasons[category] ?? "Not detected.";
}

/** Looks up CI coverage metadata for a test category. */
export function ciInfoFor(category, ci) {
  return ci.perCategory[category] ?? { wired: false, blocking: false, evidence: [] };
}

/** Produces the reason a category lacks CI coverage. */
export function ciGapReasonFor(category, ciInfo, ciIncludingUntracked) {
  const note = !ciInfo.blocking && ciIncludingUntracked?.blocking
    ? " (It WOULD be gated if currently-uncommitted CI workflow files were merged; do not credit that until they are.)" : "";
  if (!ciInfo.wired) return `Detected locally but never invoked anywhere in committed CI; nothing forces it to run before merge.${note}`;
  return `Runs in CI but every matching step is non-blocking (continue-on-error/allow_failure); a failure here is invisible, not gated.${note}`;
}
