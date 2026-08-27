import { scanDomainApiRules } from "./generic-common-source-ownership-architecture-api.mjs";
import { scanDomainStructureRules } from "./generic-common-source-ownership-architecture-structure.mjs";

/** Appends domain, architecture, and source-ownership findings for one file. */
export function scanDomainAndArchitectureRules(violations, root, filePath, rel, lines, text, importText) {
  scanDomainStructureRules(violations, root, filePath, rel, lines, text, importText);
  scanDomainApiRules(violations, root, filePath, rel, lines, text, importText);
}
