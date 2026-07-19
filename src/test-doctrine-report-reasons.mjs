export function reasonFor(category, nature) {
  const moneyNote = nature.hasMoneyCriticalSurface
    ? ` This project has money/billing-looking files (${nature.moneyCriticalFiles.slice(0, 3).join(", ")}) â€” treat idempotency/invariant coverage here as higher priority.`
    : "";
  const clientNote = nature.hasMultiServiceBoundary
    ? ` Candidate boundary files: ${nature.multiServiceClientFiles.slice(0, 3).join(", ")}.`
    : "";
  const REASONS = {
    integration: `Project looks like a web API or async worker â€” integration tests (real DB/app, not mocks) verify the actual request/consumer lifecycle, not just isolated functions.${moneyNote}`,
    e2e: "A frontend UI was detected â€” no end-to-end suite means UI regressions can only be caught by hand.",
    contract: `A public API or internal service boundary was detected â€” without contract tests, either side can silently break the other.${clientNote}`,
    security: "Baseline secret-scanning and static-analysis tooling protects against the cheapest, most common class of incidents.",
    coverageTooling: "No coverage measurement tool found â€” you can't tell what's actually exercised.",
    propertyFuzzing: "No property-based or API-fuzz tooling found â€” hand-written examples miss edge cases a generator would find for free.",
    loadPerformance: "No load/perf tooling found â€” capacity and degradation behavior are untested.",
    concurrencyRaceTests: `No tests found exercising parallel/concurrent requests â€” race conditions (double-processing, lost updates) only show up under real concurrency, never in sequential tests.${moneyNote}`,
    idempotencyReplayTests: `No tests found asserting that repeating a request doesn't repeat its effect â€” without this, retries and duplicate deliveries are unverified.${moneyNote}`,
    rollbackCompensationTests: "No tests found for rollback/compensation logic â€” partial-failure recovery paths are exactly where bugs hide because they're rarely exercised.",
    timeClockTests: "No tests found manipulating time/clock (freezegun, synthetic timers) â€” expiry, cooldown, and scheduling logic is untested against clock skew or boundary timing.",
    economicInvariantTests: `Money-critical files were detected but no tests assert an explicit invariant (balance conservation, no double-charge) â€” these are the tests that catch a state bug before it costs money.${moneyNote}`,
    killSwitchTests: "Money-critical files were detected but no tests exercise a kill-switch/circuit-breaker/emergency-disable path â€” an untested emergency control is a control you can't trust when you need it.",
  };
  return REASONS[category] ?? "Not detected.";
}

export function ciGapReasonFor(category, ciInfo, ciInfoIncludingUntracked) {
  const uncommittedNote = !ciInfo.blocking && ciInfoIncludingUntracked?.blocking
    ? " (It WOULD be gated if currently-uncommitted CI workflow files were merged â€” don't credit that until they are.)"
    : "";
  if (!ciInfo.wired) {
    return `Detected locally but never invoked anywhere in committed CI â€” nothing forces it to run before merge.${uncommittedNote}`;
  }
  return `Runs in CI but every matching step is non-blocking (continue-on-error/allow_failure) â€” a failure here is invisible, not gated.${uncommittedNote}`;
}
