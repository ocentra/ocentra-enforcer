import fs from "node:fs";
import path from "node:path";
import { normalizeRel } from "./path-utils.mjs";
import { countLines, finding } from "../scripts/check-source-core.mjs";

export function collectAiRuleIndexStandaloneFindings(root, config) {
  const findings = [];
  const agentsPath = path.join(root, "AGENTS.md");
  const rulesRoot = path.join(root, ".ocentra-ai", "rules");
  if (!fs.existsSync(agentsPath) || !fs.existsSync(rulesRoot)) return findings;

  const ruleFiles = fs
    .readdirSync(rulesRoot)
    .filter((entry) => entry.endsWith(".md") || entry.endsWith(".mdc"))
    .map((entry) => path.join(rulesRoot, entry));
  const indexFile =
    ruleFiles.find((file) => /rules|index/iu.test(path.basename(file))) ??
    ruleFiles[0];
  if (!indexFile) return findings;

  const agentsText = fs.readFileSync(agentsPath, "utf8");
  const indexText = fs.readFileSync(indexFile, "utf8");
  const indexRel = normalizeRel(root, indexFile);
  if (
    !agentsText.includes(indexRel) &&
    !agentsText.includes(indexRel.replaceAll("/", "\\"))
  ) {
    findings.push(
      finding(
        root,
        agentsPath,
        1,
        "AI-1.1",
        `AGENTS.md must reference ${indexRel}`,
        null,
      ),
    );
  }

  for (const ruleFile of ruleFiles) {
    const rel = normalizeRel(root, ruleFile);
    const lineCount = countLines(fs.readFileSync(ruleFile, "utf8"));
    if (
      ruleFile !== indexFile &&
      !indexText.includes(normalizeRel(rulesRoot, ruleFile))
    ) {
      findings.push(
        finding(
          root,
          ruleFile,
          1,
          "AI-1.1",
          `${rel} is not linked from ${indexRel}`,
          null,
        ),
      );
    }
    const maxLines = config.agentRuleMaxLines ?? 220;
    if (lineCount > maxLines) {
      findings.push(
        finding(
          root,
          ruleFile,
          maxLines + 1,
          "AI-1.1",
          `${rel} has ${lineCount} lines; split rule files above ${maxLines}`,
          null,
        ),
      );
    }
  }
  return findings;
}
