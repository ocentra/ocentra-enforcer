const RUST_ROUTE_FAMILIES = [
  "source",
  "domain",
  "imports-modules",
  "async-runtime",
];
const RUST_MANIFEST_FAMILIES = ["toolchain-cargo", "dependencies"];
const TYPESCRIPT_SOURCE_FAMILIES = [
  "source",
  "imports-modules",
  "generated-artifacts",
];
const PYTHON_SOURCE_FAMILIES = ["source", "tests"];

const WORKSPACE_ROUTE_SPEC = {
  commonFamilies: new Set(["documentation", "security", "source"]),
  languageFamilies: new Set([...RUST_ROUTE_FAMILIES, ...RUST_MANIFEST_FAMILIES]),
  languages: new Set(["rust"]),
};

const FILE_ROUTE_SPECS = {
  "rust-source": {
    commonFamilies: ["documentation", "security", "source"],
    languageFamilies: RUST_ROUTE_FAMILIES,
    languages: ["rust"],
  },
  "rust-manifest": {
    commonFamilies: ["security"],
    languageFamilies: RUST_MANIFEST_FAMILIES,
    languages: ["rust"],
  },
  "rust-dependencies": {
    commonFamilies: ["security"],
    languageFamilies: ["dependencies"],
    languages: ["rust"],
  },
  "rust-tooling": {
    commonFamilies: ["security"],
    languageFamilies: ["toolchain-cargo"],
    languages: ["rust"],
  },
  "typescript-source": {
    commonFamilies: ["source"],
    languageFamilies: TYPESCRIPT_SOURCE_FAMILIES,
    languages: ["typescript"],
  },
  "typescript-test": {
    commonFamilies: ["source"],
    languageFamilies: [...TYPESCRIPT_SOURCE_FAMILIES, "tests"],
    languages: ["typescript"],
  },
  "python-source": {
    commonFamilies: ["source"],
    languageFamilies: ["source"],
    languages: ["python"],
  },
  "python-test": {
    commonFamilies: ["source"],
    languageFamilies: PYTHON_SOURCE_FAMILIES,
    languages: ["python"],
  },
};

const FILE_KIND_BY_NAME = new Map([
  ["Cargo.toml", "rust-manifest"],
  ["Cargo.lock", "rust-dependencies"],
  ["deny.toml", "rust-dependencies"],
  ["rust-toolchain.toml", "rust-tooling"],
  ["clippy.toml", "rust-tooling"],
  ["rustfmt.toml", "rust-tooling"],
]);

const TYPESCRIPT_FILE_PATTERN = /\.(?:ts|tsx|mts|cts|js|jsx|mjs|cjs)$/u;

export function buildRouteSpec(args) {
  return isWorkspaceScope(args)
    ? cloneRouteSpec(WORKSPACE_ROUTE_SPEC)
    : collectFileRouteSpec(args.files ?? []);
}

function collectFileRouteSpec(files) {
  const routeSpec = emptyRouteSpec();
  for (const file of normalizeFiles(files)) {
    mergeRouteSpec(routeSpec, specForFile(file));
  }
  if (routeSpec.languages.size === 0) {
    routeSpec.languages.add("common");
  }
  return routeSpec;
}

function specForFile(file) {
  const fileKind = routeKindForFile(file);
  return fileKind ? FILE_ROUTE_SPECS[fileKind] ?? emptyRouteSpec() : emptyRouteSpec();
}

function routeKindForFile(file) {
  const normalized = file.split(/[\\/]+/u).pop() ?? file;
  if (file.endsWith(".rs")) return "rust-source";
  if (FILE_KIND_BY_NAME.has(normalized)) return FILE_KIND_BY_NAME.get(normalized);
  if (TYPESCRIPT_FILE_PATTERN.test(file)) {
    return isTestFile(file) ? "typescript-test" : "typescript-source";
  }
  if (file.endsWith(".py")) {
    return isTestFile(file) ? "python-test" : "python-source";
  }
  return null;
}

function normalizeFiles(files) {
  return Array.isArray(files) ? files : [];
}

function isWorkspaceScope(args) {
  return args.scope === "crate" || args.scope === "workspace";
}

function emptyRouteSpec() {
  return {
    commonFamilies: new Set(),
    languageFamilies: new Set(),
    languages: new Set(),
  };
}

function cloneRouteSpec(routeSpec) {
  return {
    commonFamilies: new Set(routeSpec.commonFamilies),
    languageFamilies: new Set(routeSpec.languageFamilies),
    languages: new Set(routeSpec.languages),
  };
}

function mergeRouteSpec(target, source) {
  mergeSet(target.commonFamilies, source.commonFamilies);
  mergeSet(target.languageFamilies, source.languageFamilies);
  mergeSet(target.languages, source.languages);
}

function mergeSet(target, values) {
  for (const value of values) {
    target.add(value);
  }
}

function isTestFile(file) {
  return (
    /(?:^|[\\/])tests?(?:[\\/]|$)/u.test(file) ||
    /\.(?:test|spec)\.[^.]+$/u.test(file)
  );
}

