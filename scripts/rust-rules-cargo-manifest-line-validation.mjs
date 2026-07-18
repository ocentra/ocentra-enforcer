import { addViolation, contextHas } from "./rust-rules-path-core.mjs";
import { isAllowedPathDependency } from "./rust-rules-cargo-manifest-path-dependencies.mjs";
import { scanManifestDependencyPolicy } from "./rust-rules-cargo-manifest-dependency-policy.mjs";
import { dependencyNameFromManifestLine, dependencyRequirementFromManifestLine } from "./rust-rules-cargo-manifest-identity.mjs";

/** Checks individual Cargo manifest lines against structural policy. */
export function scanCargoManifestLines(context) {
  const state = { section: "", namesBySection: new Map(), requirementsByName: new Map() };
  context.lines.forEach((line, index) => scanCargoManifestLine({ ...context, line, index, state }));
  return state;
}

function scanCargoManifestLine(context) {
  updateSection(context);
  addWildcardFinding(context);
  const dependency = dependencyContext(context);
  if (dependency.name && dependency.inSection) scanDependency(context, dependency);
  addLooseVersionFinding(context, dependency);
  addGitFinding(context);
  addPathFinding(context, dependency);
  addBuildDependencyFinding(context);
}

function updateSection({ line, state }) {
  const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*$/u);
  if (sectionMatch) state.section = sectionMatch[1];
}

function addWildcardFinding({ violations, root, manifest, line, index }) {
  if (/version\s*=\s*"\*"|=\s*"\*"/u.test(line)) addViolation(violations, root, manifest, index + 1, "RR-9.1", "Wildcard dependency versions are forbidden.", line);
}

function dependencyContext({ line, state }) {
  const inSection = /^(?:dependencies|dev-dependencies|build-dependencies|target\..+\.dependencies)(?:\.|$)/u.test(state.section);
  return { name: dependencyNameFromManifestLine(line), inSection, inProduction: /^(?:dependencies|target\..+\.dependencies)(?:\.|$)/u.test(state.section) };
}

function scanDependency(context, dependency) {
  recordDependency(context, dependency);
  scanManifestDependencyPolicy({ ...context, dependencyName: dependency.name, inProductionDependencySection: dependency.inProduction, currentSection: context.state.section, workspacePackageNames: context.workspacePackageNames });
}

function recordDependency({ state, line }, dependency) {
  if (!state.namesBySection.has(state.section)) state.namesBySection.set(state.section, new Set());
  state.namesBySection.get(state.section).add(dependency.name);
  const requirement = dependencyRequirementFromManifestLine(line);
  if (dependency.inProduction && requirement) addRequirement(state.requirementsByName, dependency.name, requirement);
}

function addRequirement(requirementsByName, dependencyName, requirement) {
  if (!requirementsByName.has(dependencyName)) requirementsByName.set(dependencyName, new Set());
  requirementsByName.get(dependencyName).add(requirement);
}

function addLooseVersionFinding({ violations, root, manifest, line, index }, dependency) {
  const loose = /=\s*"(?:>=|>|<=|<)[^"]*"|=\s*"\d+"|version\s*=\s*"(?:>=|>|<=|<)[^"]*"|version\s*=\s*"\d+"/u.test(line);
  if (dependency.inSection && dependency.name !== null && loose) addViolation(violations, root, manifest, index + 1, "RR-9.16", "Loose dependency version range found.", line);
}

function addGitFinding({ violations, root, manifest, line, index, config }) {
  if (!config.allowGitDependencies && /\bgit\s*=/u.test(line)) addViolation(violations, root, manifest, index + 1, "RR-9.2", "Git dependency found.", line);
}

function addPathFinding(context, dependency) {
  const hasForbiddenPath = !context.config.allowPathDependencies && dependency.inSection && /\bpath\s*=/u.test(context.line) && !isAllowedPathDependency({ root: context.root, manifest: context.manifest, dependencyName: dependency.name, line: context.line, currentSection: context.state.section, workspacePackageNames: context.workspacePackageNames });
  if (hasForbiddenPath) addViolation(context.violations, context.root, context.manifest, context.index + 1, "RR-9.3", "Path dependency found.", context.line);
}

function addBuildDependencyFinding({ violations, root, manifest, line, index, lines, state }) {
  const dependencyLine = /^\s*[\w.-]+\s*=/u.test(line);
  if (/^build-dependencies(?:\.|$)/u.test(state.section) && dependencyLine && !contextHas(lines, index, "BUILD-DEPENDENCY-JUSTIFICATION:", 4)) addViolation(violations, root, manifest, index + 1, "RR-9.30", "build-dependency lacks BUILD-DEPENDENCY-JUSTIFICATION.", line);
}
