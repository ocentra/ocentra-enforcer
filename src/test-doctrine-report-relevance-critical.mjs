/** Returns critical relevance for a category/nature pair when applicable. */
export function criticalRelevance(category, nature) {
  const moneyTier = nature.hasMoneyCriticalSurface ? "core" : "suggested";
  switch (category) {
    case "concurrencyRaceTests": return { relevant: [nature.isWebApi, nature.hasAsyncWorkers].some(Boolean), tier: moneyTier };
    case "idempotencyReplayTests": return { relevant: [nature.isWebApi, nature.hasMultiServiceBoundary, nature.hasMoneyCriticalSurface].some(Boolean), tier: moneyTier };
    case "rollbackCompensationTests": return { relevant: [nature.hasMoneyCriticalSurface, nature.hasAsyncWorkers].some(Boolean), tier: moneyTier };
    case "timeClockTests": return { relevant: true, tier: nature.hasMoneyCriticalSurface ? "suggested" : "optional" };
    case "economicInvariantTests": return { relevant: nature.hasMoneyCriticalSurface, tier: "core" };
    case "killSwitchTests": return { relevant: nature.hasMoneyCriticalSurface, tier: "suggested" };
    default: return null;
  }
}
