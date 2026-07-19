import { riskRelevance } from "./test-doctrine-report-risk-relevance.mjs";

const STATIC_RELEVANCE = {
  unit: { relevant: true, tier: "core" },
  security: { relevant: true, tier: "core" },
  coverageTooling: { relevant: true, tier: "core" },
  propertyFuzzing: { relevant: true, tier: "suggested" },
  mutation: { relevant: true, tier: "optional" },
  snapshot: { relevant: true, tier: "optional" },
};

const CONTEXTUAL_RELEVANCE = {
  integration: (nature) => ({
    relevant: nature.isWebApi || nature.hasAsyncWorkers,
    tier: "core",
  }),
  e2e: (nature) => ({ relevant: nature.hasFrontendUi, tier: "core" }),
  contract: (nature) => ({
    relevant: nature.isWebApi || nature.hasMultiServiceBoundary,
    tier: "core",
  }),
  loadPerformance: (nature) => ({
    relevant: nature.isWebApi || nature.hasAsyncWorkers,
    tier: "suggested",
  }),
};

/** Resolves doctrine relevance and priority for one category. */
export function relevance(category, nature) {
  const staticResult = STATIC_RELEVANCE[category];
  if (staticResult) return staticResult;
  const contextual = CONTEXTUAL_RELEVANCE[category];
  if (contextual) return contextual(nature);
  return riskRelevance(category, nature);
}
