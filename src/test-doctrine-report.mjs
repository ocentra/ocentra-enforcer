/* Builds the test-doctrine gap report from category and CI evidence. */
import { CATEGORY_SIGNALS } from "./test-doctrine-signals.mjs";
import {
  CATEGORY_LABELS,
  detectCategory,
} from "./test-doctrine-report-categories.mjs";
import { relevance } from "./test-doctrine-report-relevance.mjs";
import {
  ciGapReasonFor,
  ciInfoFor,
  reasonFor,
} from "./test-doctrine-report-reasons.mjs";

function buildReport({ root, relPaths, manifestText, nature, ci, files }) {
  const detected = {};
  const missing = [];
  const ciGaps = [];
  for (const category of Object.keys(CATEGORY_SIGNALS)) {
    const result = detectCategory(category, relPaths, manifestText, files);
    const categoryRelevance = relevance(category, nature);
    const ciInfo = ciInfoFor(category, ci);
    const ciIncludingUntracked = ci.perCategoryIncludingUntracked?.[category] ?? null;
    detected[category] = {
      label: CATEGORY_LABELS[category],
      ...result,
      relevant: categoryRelevance.relevant,
      ci: ciInfo,
      ciIncludingUntracked,
    };
    if (categoryRelevance.relevant && !result.present) {
      missing.push({
        category,
        label: CATEGORY_LABELS[category],
        tier: categoryRelevance.tier,
        reason: reasonFor(category, nature),
      });
    } else if (categoryRelevance.relevant && result.present && ciInfo && ciInfo.blocking !== true) {
      ciGaps.push({
        category,
        label: CATEGORY_LABELS[category],
        reason: ciGapReasonFor(category, ciInfo, ciIncludingUntracked),
        ciEvidence: ciInfo.evidence,
      });
    }
  }
  const tierOrder = { core: 0, suggested: 1, optional: 2 };
  missing.sort((left, right) => tierOrder[left.tier] - tierOrder[right.tier]);
  return {
    root,
    caveat: "Heuristic, signal-based (file names, config files, dependency manifests, CI step text); not a certification. "
      + "Evidence should be opened and judged, not trusted at face value; absence of a signal does not always mean absence of the practice.",
    nature,
    ciConfigFilesFound: ci.ciConfigFilesFound,
    hasUntrackedCiFiles: ci.hasUntrackedCiFiles ?? false,
    detected,
    missing,
    ciGaps,
    summary: {
      categoriesRelevant: Object.values(detected).filter((item) => item.relevant).length,
      categoriesPresent: Object.values(detected).filter((item) => item.relevant && item.present).length,
      categoriesMissing: missing.length,
      coreMissing: missing.filter((item) => item.tier === "core").length,
      ciGaps: ciGaps.length,
    },
  };
}

export { buildReport, CATEGORY_LABELS };
