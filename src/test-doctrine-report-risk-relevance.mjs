const RISK_RELEVANCE = {
  concurrencyRaceTests: (nature) => ({
    relevant: nature.isWebApi || nature.hasAsyncWorkers,
    tier: nature.hasMoneyCriticalSurface ? "core" : "suggested",
  }),
  idempotencyReplayTests: (nature) => ({
    relevant:
      nature.isWebApi ||
      nature.hasMultiServiceBoundary ||
      nature.hasMoneyCriticalSurface,
    tier: nature.hasMoneyCriticalSurface ? "core" : "suggested",
  }),
  rollbackCompensationTests: (nature) => ({
    relevant: nature.hasMoneyCriticalSurface || nature.hasAsyncWorkers,
    tier: nature.hasMoneyCriticalSurface ? "core" : "suggested",
  }),
  timeClockTests: (nature) => ({
    relevant: true,
    tier: nature.hasMoneyCriticalSurface ? "suggested" : "optional",
  }),
  economicInvariantTests: (nature) => ({
    relevant: nature.hasMoneyCriticalSurface,
    tier: "core",
  }),
  killSwitchTests: (nature) => ({
    relevant: nature.hasMoneyCriticalSurface,
    tier: "suggested",
  }),
};

/** Resolves risk-shaped category relevance or the default optional result. */
export function riskRelevance(category, nature) {
  const resolver = RISK_RELEVANCE[category];
  return resolver
    ? resolver(nature)
    : { relevant: false, tier: "optional" };
}
