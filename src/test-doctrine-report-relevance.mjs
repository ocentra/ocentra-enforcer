const CORE_ALWAYS = new Set(["unit", "security", "coverageTooling"]);
const OPTIONAL_ALWAYS = new Set(["mutation", "snapshot"]);
import { criticalRelevance } from "./test-doctrine-report-relevance-critical.mjs";

/** Returns the relevance classification for a category/nature pair. */
export function relevance(category, nature) {
  if (CORE_ALWAYS.has(category)) return { relevant: true, tier: "core" };
  if (OPTIONAL_ALWAYS.has(category)) return { relevant: true, tier: "optional" };
  const critical = criticalRelevance(category, nature);
  if (critical) return critical;
  switch (category) {
    case "integration": return { relevant: [nature.isWebApi, nature.hasAsyncWorkers].some(Boolean), tier: "core" };
    case "e2e": return { relevant: nature.hasFrontendUi, tier: "core" };
    case "contract": return { relevant: [nature.isWebApi, nature.hasMultiServiceBoundary].some(Boolean), tier: "core" };
    case "propertyFuzzing": return { relevant: true, tier: "suggested" };
    case "loadPerformance": return { relevant: [nature.isWebApi, nature.hasAsyncWorkers].some(Boolean), tier: "suggested" };
    default: return { relevant: false, tier: "optional" };
  }
}
