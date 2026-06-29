#!/usr/bin/env node
/*
 * Ocentra Enforcer Rust hard gate.
 * Cross-platform Node.js validator with Effect Schema validated external inputs.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  ProductName,
  decodeEnforcerConfig,
  decodeInitRequest,
  decodeRuleRegistry,
} from '../schemas/effect/enforcer-schemas.mjs';

const RULES = {
  'RR-1.1': {
    title: 'Rust toolchain must be pinned',
    snippet: 'Add rust-toolchain.toml with channel and rustfmt/clippy components.',
  },
  'RR-1.2': {
    title: 'Cargo.lock must exist for deterministic builds',
    snippet:
      'Run cargo generate-lockfile and commit Cargo.lock unless this is a publish-only library with a documented exception.',
  },
  'RR-1.3': {
    title: 'Clippy configuration must exist',
    snippet: 'Add clippy.toml to centralize lint thresholds and future policy knobs.',
  },
  'RR-1.4': {
    title: 'Dependency policy must exist',
    snippet: 'Add deny.toml and run cargo deny check in CI.',
  },
  'RR-1.5': {
    title: 'Cargo.toml must declare rust-version',
    snippet: 'Set package.rust-version or workspace.package.rust-version.',
  },
  'RR-2.1': {
    title: 'No lint suppression attributes',
    snippet:
      'Do not use #[allow(...)] or #[expect(...)]. Fix the code or strengthen the rule instead of suppressing it.',
  },
  'RR-2.2': {
    title: 'No validator suppression comments',
    snippet: 'Comments such as rust-rules: ignore/allow/skip/disable are forbidden.',
  },
  'RR-3.1': {
    title: 'Unsafe Rust is forbidden by default',
    snippet:
      'Use safe Rust. If unsafe is truly required, isolate it in an approved FFI/sys crate and set allowUnsafeCode with owner review.',
  },
  'RR-3.2': {
    title: 'Unsafe code requires SAFETY documentation',
    snippet: 'Every unsafe block requires a nearby // SAFETY: comment explaining the invariants.',
  },
  'RR-3.3': {
    title: 'Unsafe functions require a # Safety section',
    snippet: 'Document caller obligations under a rustdoc # Safety section.',
  },
  'RR-3.4': {
    title: 'Raw pointers are forbidden in public/domain APIs',
    snippet: 'Use references, NonNull wrappers, or isolated FFI boundary types.',
  },
  'RR-4.1': {
    title: 'No unwrap/expect',
    snippet: 'Replace .unwrap()/.expect() with ?, match, ok_or_else, or a typed error.',
  },
  'RR-4.2': {
    title: 'No panic-like macros in production paths',
    snippet:
      'Replace panic!/todo!/unimplemented!/unreachable! with Result, typed errors, or explicit state modelling.',
  },
  'RR-4.3': {
    title: 'No debug/console macros in Rust logic',
    snippet: 'Use tracing/logging abstractions. dbg!, println!, and eprintln! are not allowed in source logic.',
  },
  'RR-4.4': {
    title: 'Domain code must not use erased application errors',
    snippet: 'Use a typed error enum. anyhow::Result and Box<dyn Error> belong only at application boundaries.',
  },
  'RR-5.1': {
    title: 'Clone must be justified',
    snippet: 'Add a nearby CLONE-JUSTIFICATION: comment or refactor to borrow, move, Arc, or Cow.',
  },
  'RR-5.2': {
    title: 'String allocation must be justified',
    snippet: 'Add ALLOC-JUSTIFICATION: near to_string()/to_owned() or avoid allocation with borrowing/domain types.',
  },
  'RR-5.3': {
    title: 'Unchecked indexing is forbidden',
    snippet: 'Use get(), get_mut(), split_at_checked-style logic, or typed indexes that prove bounds.',
  },
  'RR-5.4': {
    title: 'Lossy casts require justification',
    snippet: 'Use TryFrom/TryInto or add CAST-JUSTIFICATION: explaining why the cast is safe.',
  },
  'RR-6.1': {
    title: 'No raw string types in domain function signatures',
    snippet:
      'Replace String/&str/str/Cow<str>/PathBuf/OsString with branded domain types. Raw text may exist only in configured boundary or owner modules.',
  },
  'RR-6.2': {
    title: 'No unbranded primitive types in domain function signatures',
    snippet:
      'Replace bool/numeric primitive parameters and returns with newtypes, enums, NonZero types, or domain value objects.',
  },
  'RR-6.3': {
    title: 'No public raw fields',
    snippet: 'Public fields must expose branded/domain types, not String, &str, bool, numbers, Vec<String>, or HashMap<String, _>.',
  },
  'RR-6.4': {
    title: 'Raw private fields require brand invariants',
    snippet:
      'Private raw fields inside domain types require a nearby BRAND-INVARIANT: comment explaining validation and meaning.',
  },
  'RR-6.5': {
    title: 'Type aliases must not disguise raw primitives',
    snippet: 'Use tuple/newtype structs instead of type UserId = String or type Count = usize.',
  },
  'RR-6.6': {
    title: 'Tuple newtypes must not expose raw inner fields',
    snippet: 'Use pub struct UserId(String); not pub struct UserId(pub String); and validate construction.',
  },
  'RR-6.26': {
    title: 'Serialized domain fields must not expose raw identity primitives',
    snippet: 'Use typed domain newtypes or enums for public serialized id/ref/event/command fields.',
  },
  'RR-7.1': {
    title: 'Wildcard imports are forbidden',
    snippet: 'Replace use x::* with explicit imports.',
  },
  'RR-7.2': {
    title: 'Wildcard public re-exports are forbidden',
    snippet: 'Replace pub use x::* with direct module imports or explicit call-site paths.',
  },
  'RR-7.3': {
    title: 'Public re-exports must match project policy',
    snippet:
      'For strict projects, remove pub use entirely. For facade projects, keep pub use only in configured facade files.',
  },
  'RR-7.4': {
    title: 'Dumping-ground modules are forbidden',
    snippet: 'Rename utils/helpers/common/misc/shared/stuff to a domain-specific module.',
  },
  'RR-7.5': {
    title: 'Build scripts are forbidden by default',
    snippet: 'Remove build.rs or explicitly approve it in config with a deterministic justification.',
  },
  'RR-8.1': {
    title: 'Blocking primitives are forbidden inside async modules',
    snippet: 'Do not combine async/.await with std::sync locks, std::thread::sleep, or blocking std::fs/std::net calls.',
  },
  'RR-8.2': {
    title: 'C-style index loops are forbidden',
    snippet: 'Use iterators, enumerate(), chunks(), windows(), or typed ranges.',
  },
  'RR-9.1': {
    title: 'Dependency versions must be pinned semver requirements',
    snippet: 'Do not use dependency version = "*".',
  },
  'RR-9.2': {
    title: 'Git dependencies are forbidden by default',
    snippet: 'Use crates.io versions or explicitly approve a pinned git revision in dependency policy.',
  },
  'RR-9.3': {
    title: 'Path dependencies are forbidden outside policy',
    snippet: 'Use workspace members or explicitly approve path dependencies.',
  },
  'RR-9.4': {
    title: 'Workspace path dependencies must stay inside the workspace',
    snippet: 'Move path dependencies under the workspace root or publish/version the dependency.',
  },
  'RR-9.5': {
    title: 'Direct dependency versions must not drift',
    snippet: 'Align direct registry dependency requirements across changed manifests.',
  },
  'RR-10.1': {
    title: 'cargo fmt must pass',
    snippet: 'Run cargo fmt --all -- --check or the scoped package equivalent before completing the task.',
  },
  'RR-10.2': {
    title: 'cargo clippy hard mode must pass',
    snippet: 'Run cargo clippy with -D warnings and selected deny lints.',
  },
  'RR-10.3': {
    title: 'cargo test must pass',
    snippet: 'Run cargo test for the workspace or touched crate.',
  },
  'RR-10.4': {
    title: 'rustdoc warnings must be errors',
    snippet: 'Run cargo doc with RUSTDOCFLAGS denying rustdoc warnings for release/readiness gates.',
  },
  'RR-11.1': {
    title: 'cargo-deny must pass',
    snippet: 'Run cargo deny check to enforce advisories, bans, licenses, and source policy.',
  },
  'RR-11.2': {
    title: 'cargo-deny must be installed when dependency policy is required',
    snippet: 'Install with cargo install --locked cargo-deny or disable only in local scan mode.',
  },
  'RR-11.3': {
    title: 'cargo-audit must pass when enabled',
    snippet: 'Run cargo audit and upgrade vulnerable crates instead of ignoring advisories.',
  },
  'RR-18.16': {
    title: 'Runtime Rust source must not contain inline string literals',
    snippet: 'Move stable runtime strings to constants, schemas, protocol crates, or configured owner modules.',
  },
};

const DEFAULT_CONFIG = Object.freeze({
  schemaVersion: 2,
  profileName: 'strict',
  failFast: false,
  enforceWorkspaceFiles: true,
  requireCargoDeny: true,
  requireCargoAudit: false,
  runCargoDoc: false,
  cargoOnFileScope: false,
  cargoOnDiffScope: false,
  cargoTestThreads: null,
  allowUnsafeCode: false,
  allowBuildRs: false,
  allowGitDependencies: false,
  allowPathDependencies: false,
  publicReexportPolicy: 'forbid',
  ignoreDirs: ['.git', '.hub', '.turbo', 'target', 'node_modules', '.next', 'dist', 'build', 'coverage', 'output'],
  ignoreFileGlobs: [],
  rustRoots: ['src', 'crates', 'tools'],
  crateRootGlobs: ['crates/*', 'tools/*', '.'],
  testFileGlobs: [
    '**/tests/**',
    '**/*_test.rs',
    '**/*_tests.rs',
    '**/*_test_support.rs',
    '**/*_test_fixture.rs',
    '**/*_test_fixtures.rs',
  ],
  rawTypeBoundaryGlobs: [
    'src/bin/**',
    'src/main.rs',
    'src/**/boundary/**',
    'src/**/boundaries/**',
    'src/**/adapter/**',
    'src/**/adapters/**',
    'src/**/ffi/**',
    'src/**/serde/**',
    'src/**/transport/**',
  ],
  facadeFileGlobs: ['src/lib.rs', 'src/api.rs', 'src/prelude.rs', 'src/**/api.rs', 'src/**/prelude.rs'],
  rawStringOwnerGlobs: [],
  domainPrimitiveOwnerGlobs: [],
  enforceRuntimeStringLiterals: false,
  runtimeStringOwnerGlobs: [],
  runtimeStringLineAllowPatterns: [
    'env!\\(',
    '#\\[tokio::main',
    '#\\[serde',
    'serde\\(',
    'cfg\\(',
    '#\\[path\\s*=',
    'panic!\\(',
    'format!\\(',
    '^\\s*(?:pub\\s+)?const\\s+[A-Z0-9_]+\\s*:\\s*&str\\s*=\\s*"',
  ],
  enforceSerializedPublicDomainPrimitives: false,
  serializedDomainOwnerGlobs: [],
  blockedProtocolDependencies: {},
  runtimeCrates: [],
  testOnlyCrates: ['criterion', 'mockall', 'pretty_assertions', 'proptest', 'rstest', 'wiremock'],
  allowedGitDependencies: [],
});

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), '..'));
const RULE_REGISTRY_PATH = path.join(PACK_ROOT, 'rules', 'rules.json');
const DEFAULT_INIT_ADAPTERS = ['codex', 'mcp', 'precommit', 'github-actions'];
const GITHUB_ACTIONS_ADAPTERS = ['github-actions', 'codeql', 'dependency-policy', 'secret-scan', 'sbom'];
let cachedRuleDocs = null;

function usage() {
  return `Ocentra Enforcer Rust hard gate

Usage:
  ocentra-enforcer init --root <repo> --profile <profile> --adapters codex,mcp,precommit,github-actions
  ocentra-enforcer scan [options]
  ocentra-enforcer cargo [options]
  ocentra-enforcer doctor [options]
  ocentra-enforcer explain <RULE_ID>

Compatibility aliases:
  rust-rules scan [options]
  rust-rules cargo [options]
  rust-rules doctor [options]
  rust-rules explain <RULE_ID>

Scope options:
  --files <path...>       Scan explicit files or directories.
  --crate <name>          Scan one Cargo package by package.name.
  --workspace, --all      Scan the whole workspace/repo.
  --base <sha> --head <sha>
                          Scan changed files between two git refs.

Common options:
  --root <path>           Repository root. Defaults to current directory.
  --config <path>         Optional config path. Defaults to ocentra-enforcer.config.json, then rust-rules.config.json.
  --profile <name>        Named profile for init output.
  --scan-only             With scan/cargo compatibility: skip Cargo commands.
  --json                  Print machine-readable JSON report.
  --help                  Show this help.

Init options:
  --adapters <list>       Comma-separated adapters: codex,mcp,precommit,github-actions,husky,lefthook,codeql,dependency-policy,secret-scan,sbom.
  --dry-run               Print exact file plan without writing.
  --force                 Allow init to overwrite existing target files.
`;
}

function defaultArgs() {
  return {
    command: 'scan',
    root: process.cwd(),
    configPath: null,
    scanOnly: false,
    json: false,
    help: false,
    explainRuleId: null,
    profile: 'strict',
    adapters: null,
    dryRun: false,
    force: false,
    scope: { mode: 'all' },
  };
}

function parseArgs(argv) {
  const args = defaultArgs();
  const tokens = argv.slice(2);
  if (tokens[0] && ['init', 'scan', 'cargo', 'doctor', 'explain'].includes(tokens[0])) {
    args.command = tokens.shift();
  }

  if (args.command === 'explain') {
    args.explainRuleId = tokens.shift() ?? null;
  }

  const explicitFiles = [];
  for (let i = 0; i < tokens.length; i += 1) {
    const arg = tokens[i];
    if (arg === '--root') {
      args.root = tokens[++i];
    } else if (arg === '--config') {
      args.configPath = tokens[++i];
    } else if (arg === '--profile') {
      args.profile = tokens[++i];
    } else if (arg === '--adapters') {
      args.adapters = parseAdapterList(tokens[++i] ?? '');
    } else if (arg === '--dry-run') {
      args.dryRun = true;
    } else if (arg === '--force') {
      args.force = true;
    } else if (arg === '--scan-only') {
      args.scanOnly = true;
    } else if (arg === '--json') {
      args.json = true;
    } else if (arg === '--workspace' || arg === '--all') {
      args.scope = { mode: 'all' };
    } else if (arg === '--crate' || arg === '-p' || arg === '--package') {
      args.scope = { mode: 'crate', crateName: tokens[++i] };
    } else if (arg === '--base') {
      const base = tokens[++i] ?? null;
      const current = args.scope.mode === 'diff' ? args.scope : { mode: 'diff', base: null, head: null };
      args.scope = { ...current, base };
    } else if (arg === '--head') {
      const head = tokens[++i] ?? null;
      const current = args.scope.mode === 'diff' ? args.scope : { mode: 'diff', base: null, head: null };
      args.scope = { ...current, head };
    } else if (arg === '--files') {
      for (let fileIndex = i + 1; fileIndex < tokens.length; fileIndex += 1) {
        if (tokens[fileIndex].startsWith('-')) {
          i = fileIndex - 1;
          break;
        }
        explicitFiles.push(tokens[fileIndex]);
        i = fileIndex;
      }
    } else if (arg === '--help' || arg === '-h') {
      args.help = true;
    } else if (arg.startsWith('-')) {
      throw new Error(`Unknown argument: ${arg}`);
    } else {
      explicitFiles.push(arg);
    }
  }

  if (explicitFiles.length > 0) {
    args.scope = { mode: 'files', files: explicitFiles };
  }
  if (args.scope.mode === 'diff' && (!args.scope.base || !args.scope.head)) {
    throw new Error('Diff scope requires --base <sha> --head <sha>.');
  }

  return args;
}

function loadConfig(root, explicitPath) {
  const candidate = explicitPath ? path.resolve(explicitPath) : resolveDefaultConfigPath(root);
  let user = {};
  if (fs.existsSync(candidate)) {
    user = JSON.parse(fs.readFileSync(candidate, 'utf8'));
  }
  return normalizeConfig(decodeEnforcerConfig({ ...DEFAULT_CONFIG, ...user }));
}

function resolveDefaultConfigPath(root) {
  const branded = path.join(root, 'ocentra-enforcer.config.json');
  if (fs.existsSync(branded)) return branded;
  return path.join(root, 'rust-rules.config.json');
}

function normalizeConfig(config) {
  return {
    ...config,
    runtimeStringLineAllowRegexps: config.runtimeStringLineAllowPatterns.map((pattern) => new RegExp(pattern, 'u')),
    allowedGitDependenciesSet: new Set(config.allowedGitDependencies),
    runtimeCratesSet: new Set(config.runtimeCrates),
    testOnlyCratesSet: new Set(config.testOnlyCrates),
  };
}

function parseAdapterList(value) {
  return value
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function createInitReport(args) {
  const request = decodeInitRequest({
    root: path.resolve(args.root ?? process.cwd()),
    profile: args.profile ?? 'strict',
    adapters: args.adapters ?? DEFAULT_INIT_ADAPTERS,
    dryRun: args.dryRun,
    force: args.force,
  });
  const root = path.resolve(request.root ?? process.cwd());
  const adapters = expandAdapters(root, request.adapters ?? DEFAULT_INIT_ADAPTERS);
  const writes = buildInitWrites(root, request.profile ?? 'strict', adapters, Boolean(request.force));
  return {
    ok: true,
    command: 'init',
    productName: ProductName,
    root,
    profile: request.profile ?? 'strict',
    dryRun: Boolean(request.dryRun),
    force: Boolean(request.force),
    adapters,
    files: writes,
  };
}

function expandAdapters(root, requestedAdapters) {
  const adapters = new Set(requestedAdapters.length > 0 ? requestedAdapters : DEFAULT_INIT_ADAPTERS);
  if (adapters.has('github-actions')) {
    for (const adapter of GITHUB_ACTIONS_ADAPTERS) adapters.add(adapter);
  }
  if (adapters.has('precommit') && targetUsesHusky(root)) adapters.add('husky');
  return [...adapters].sort((a, b) => a.localeCompare(b));
}

function targetUsesHusky(root) {
  if (fs.existsSync(path.join(root, '.husky'))) return true;
  const packagePath = path.join(root, 'package.json');
  if (!fs.existsSync(packagePath)) return false;
  try {
    const parsed = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
    return Boolean(parsed.devDependencies?.husky || parsed.dependencies?.husky || parsed.scripts?.prepare?.includes('husky'));
  } catch {
    return false;
  }
}

function buildInitWrites(root, profile, adapters, force) {
  const writes = [
    initWrite(root, 'core', 'ocentra-enforcer.config.json', 'generated', force, {
      content: `${JSON.stringify({ schemaVersion: 2, profileName: profile }, null, 2)}\n`,
    }),
  ];

  if (adapters.includes('codex')) {
    writes.push(
      initWrite(root, 'codex', '.codex/skills/ocentra-enforcer/SKILL.md', 'template', force, {
        source: 'skills/rust-rules-hard-gate/SKILL.md',
      })
    );
  }
  if (adapters.includes('mcp')) {
    writes.push(initWrite(root, 'mcp', '.mcp.json', 'generated', force, { content: mcpConfigTemplate() }));
  }
  if (adapters.includes('precommit')) {
    writes.push(
      initWrite(root, 'precommit', '.git/hooks/pre-commit', 'template', force, {
        source: 'adapters/git-hooks/pre-commit.sh',
      })
    );
  }
  if (adapters.includes('husky')) {
    writes.push(
      initWrite(root, 'husky', '.husky/pre-commit', 'template', force, {
        source: 'adapters/husky/pre-commit',
      })
    );
  }
  if (adapters.includes('lefthook')) {
    writes.push(
      initWrite(root, 'lefthook', 'lefthook.yml', 'template', force, {
        source: 'adapters/lefthook/lefthook.yml',
      })
    );
  }

  const workflowMap = [
    ['github-actions', '.github/workflows/ocentra-enforcer.yml', 'adapters/github-actions/ocentra-enforcer.yml'],
    ['codeql', '.github/workflows/codeql.yml', 'adapters/github-actions/codeql.yml'],
    ['dependency-policy', '.github/workflows/dependency-policy.yml', 'adapters/github-actions/dependency-policy.yml'],
    ['secret-scan', '.github/workflows/secret-scan.yml', 'adapters/github-actions/secret-scan.yml'],
    ['sbom', '.github/workflows/sbom.yml', 'adapters/github-actions/sbom.yml'],
  ];
  for (const [adapter, targetPath, source] of workflowMap) {
    if (adapters.includes(adapter)) writes.push(initWrite(root, adapter, targetPath, 'template', force, { source }));
  }

  return writes;
}

function initWrite(root, adapter, targetPath, kind, force, details) {
  const absolutePath = path.join(root, targetPath);
  return {
    adapter,
    path: targetPath,
    action: fs.existsSync(absolutePath) ? (force ? 'overwrite' : 'skip-existing') : 'write',
    kind,
    ...details,
  };
}

function mcpConfigTemplate() {
  return `${JSON.stringify(
    {
      mcpServers: {
        'ocentra-enforcer': {
          command: 'node',
          args: [path.join(PACK_ROOT, 'mcp', 'rust-rules-mcp.mjs')],
        },
      },
    },
    null,
    2
  )}\n`;
}

function applyInitReport(report) {
  for (const file of report.files) {
    if (file.action === 'skip-existing') continue;
    const targetPath = path.join(report.root, file.path);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    const content = file.content ?? fs.readFileSync(path.join(PACK_ROOT, file.source), 'utf8');
    fs.writeFileSync(targetPath, content, 'utf8');
    if (file.adapter === 'precommit' || file.adapter === 'husky') {
      try {
        fs.chmodSync(targetPath, 0o755);
      } catch {
        // Windows does not need executable bits for these generated scripts.
      }
    }
  }
}

function printInitReport(report) {
  const mode = report.dryRun ? 'dry-run' : 'write';
  console.log(`Ocentra Enforcer init ${mode} for ${report.root}`);
  console.log(`Profile: ${report.profile}`);
  console.log(`Adapters: ${report.adapters.join(', ')}`);
  for (const file of report.files) {
    console.log(`${file.action} ${file.path} (${file.adapter})`);
  }
}

function normalizeRel(root, filePath) {
  return path.relative(root, path.resolve(filePath)).split(path.sep).join('/');
}

function toPosix(value) {
  return value.split(path.sep).join('/');
}

function repoAbsolute(root, value) {
  return path.isAbsolute(value) ? value : path.join(root, value);
}

function globToRegExp(glob) {
  const special = /[.+^${}()|[\]\\]/g;
  let pattern = '';
  for (let i = 0; i < glob.length; i += 1) {
    const char = glob[i];
    if (char === '*') {
      if (glob[i + 1] === '*') {
        pattern += '.*';
        i += 1;
      } else {
        pattern += '[^/]*';
      }
    } else if (char === '?') {
      pattern += '[^/]';
    } else {
      pattern += char.replace(special, '\\$&');
    }
  }
  return new RegExp(`^${pattern}$`, 'u');
}

const globCache = new Map();

function matchesGlob(relPath, glob) {
  if (!globCache.has(glob)) {
    globCache.set(glob, globToRegExp(glob));
  }
  return globCache.get(glob).test(relPath);
}

function matchesAnyGlob(relPath, globs) {
  return globs.some((glob) => matchesGlob(relPath, glob));
}

function isIgnoredPath(relPath, config) {
  return relPath.split('/').some((segment) => config.ignoreDirs.includes(segment)) || matchesAnyGlob(relPath, config.ignoreFileGlobs);
}

function isRustFile(filePath) {
  return path.extname(filePath).toLowerCase() === '.rs';
}

function lineNumberAt(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1;
  }
  return line;
}

function runGit(root, args, label) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8', shell: false });
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) {
    throw new Error(`${label}: ${result.stderr?.trim() || 'git command failed'}`);
  }
  return result.stdout.trim();
}

function walkFiles(root, start, config, collect) {
  if (!fs.existsSync(start)) return;
  const stats = fs.statSync(start);
  const rel = normalizeRel(root, start);
  if (isIgnoredPath(rel, config)) return;
  if (stats.isDirectory()) {
    for (const entry of fs.readdirSync(start, { withFileTypes: true })) {
      walkFiles(root, path.join(start, entry.name), config, collect);
    }
    return;
  }
  if (stats.isFile()) collect(start);
}

function collectAllRustFiles(root, config) {
  const starts = config.rustRoots.map((entry) => repoAbsolute(root, entry)).filter((entry) => fs.existsSync(entry));
  if (starts.length === 0) starts.push(root);
  const files = [];
  for (const start of starts) {
    walkFiles(root, start, config, (file) => {
      if (isRustFile(file)) files.push(path.resolve(file));
    });
  }
  return uniqueSorted(files);
}

function collectExplicitRustFiles(root, config, entries) {
  const files = [];
  for (const entry of entries) {
    walkFiles(root, repoAbsolute(root, entry), config, (file) => {
      if (isRustFile(file)) files.push(path.resolve(file));
    });
  }
  return uniqueSorted(files);
}

function collectDiffRustFiles(root, config, base, head) {
  const output = runGit(root, ['diff', '--name-only', '--diff-filter=ACMR', base, head, '--', ...config.rustRoots], 'failed to list diff files');
  if (output === '') return [];
  return uniqueSorted(
    output
      .split(/\r?\n/u)
      .map((entry) => repoAbsolute(root, entry))
      .filter((entry) => fs.existsSync(entry) && isRustFile(entry) && !isIgnoredPath(normalizeRel(root, entry), config))
  );
}

function findCargoManifests(root, config) {
  const manifests = [];
  walkFiles(root, root, config, (file) => {
    if (path.basename(file) === 'Cargo.toml') manifests.push(path.resolve(file));
  });
  return uniqueSorted(manifests);
}

function packageNameFromManifest(manifestPath) {
  const text = fs.readFileSync(manifestPath, 'utf8');
  const packageBlock = text.match(/(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u);
  if (!packageBlock) return null;
  const nameMatch = packageBlock[1].match(/(?:^|\n)\s*name\s*=\s*"([^"]+)"/u);
  return nameMatch?.[1] ?? null;
}

function collectCrateRustFiles(root, config, crateName) {
  if (!crateName) throw new Error('--crate requires a package name.');
  const manifest = findCargoManifests(root, config).find((candidate) => packageNameFromManifest(candidate) === crateName);
  if (!manifest) throw new Error(`No Cargo package named "${crateName}" was found under ${root}.`);
  const crateRoot = path.dirname(manifest);
  return {
    crateName,
    crateRoot,
    manifest,
    files: collectExplicitRustFiles(root, config, [crateRoot]),
  };
}

function resolveScope(root, config, scope) {
  if (scope.mode === 'files') {
    return { ...scope, files: collectExplicitRustFiles(root, config, scope.files) };
  }
  if (scope.mode === 'diff') {
    return { ...scope, files: collectDiffRustFiles(root, config, scope.base, scope.head) };
  }
  if (scope.mode === 'crate') {
    return { ...scope, ...collectCrateRustFiles(root, config, scope.crateName) };
  }
  return { mode: 'all', files: collectAllRustFiles(root, config) };
}

function uniqueSorted(files) {
  return [...new Set(files)].sort((a, b) => a.localeCompare(b));
}

function maskRustCode(source) {
  let out = '';
  let state = 'code';
  let blockDepth = 0;
  let rawHashes = '';
  const pushMask = (ch) => {
    out += ch === '\n' ? '\n' : ' ';
  };

  for (let i = 0; i < source.length; i += 1) {
    const ch = source[i];
    const next = source[i + 1];

    if (state === 'code') {
      const rawMatch = source.slice(i).match(/^(?:b|c|br)?r(#+)?"/u);
      if (rawMatch) {
        state = 'rawString';
        rawHashes = rawMatch[1] ?? '';
        for (let j = 0; j < rawMatch[0].length; j += 1) pushMask(source[i + j]);
        i += rawMatch[0].length - 1;
        continue;
      }
      if (ch === '/' && next === '/') {
        state = 'lineComment';
        pushMask(ch);
        continue;
      }
      if (ch === '/' && next === '*') {
        state = 'blockComment';
        blockDepth = 1;
        pushMask(ch);
        continue;
      }
      if (ch === '"' || (ch === 'b' && next === '"')) {
        state = 'string';
        pushMask(ch);
        continue;
      }
      if (ch === "'") {
        if (/^'[A-Za-z_][A-Za-z0-9_]*\b/u.test(source.slice(i))) {
          out += ch;
          continue;
        }
        state = 'char';
        pushMask(ch);
        continue;
      }
      out += ch;
      continue;
    }

    if (state === 'lineComment') {
      pushMask(ch);
      if (ch === '\n') state = 'code';
      continue;
    }

    if (state === 'blockComment') {
      pushMask(ch);
      if (ch === '/' && next === '*') {
        blockDepth += 1;
        pushMask(next);
        i += 1;
      } else if (ch === '*' && next === '/') {
        blockDepth -= 1;
        pushMask(next);
        i += 1;
        if (blockDepth === 0) state = 'code';
      }
      continue;
    }

    if (state === 'string' || state === 'char') {
      pushMask(ch);
      if (ch === '\\') {
        if (i + 1 < source.length) {
          pushMask(source[i + 1]);
          i += 1;
        }
      } else if ((state === 'string' && ch === '"') || (state === 'char' && ch === "'")) {
        state = 'code';
      }
      continue;
    }

    if (state === 'rawString') {
      pushMask(ch);
      if (ch === '"' && source.slice(i + 1, i + 1 + rawHashes.length) === rawHashes) {
        for (let j = 0; j < rawHashes.length; j += 1) pushMask(source[i + 1 + j]);
        i += rawHashes.length;
        state = 'code';
      }
    }
  }

  return out;
}

function contextHas(lines, index, token, distance = 4) {
  const start = Math.max(0, index - distance);
  return lines.slice(start, index + 1).join('\n').includes(token);
}

function addViolation(violations, root, filePath, line, ruleId, detail, sourceLine = null) {
  const rule = RULES[ruleId] ?? { title: 'Unknown rule', snippet: '' };
  violations.push({
    ruleId,
    title: rule.title,
    detail,
    file: filePath === '.' ? '.' : normalizeRel(root, filePath),
    line,
    snippet: rule.snippet,
    source: sourceLine?.trim() ?? null,
  });
}

const RAW_STRING_TYPE_RE =
  /\b(?:String|str|PathBuf|OsString|CString|CStr)\b|\b(?:std|alloc)::(?:string::String|path::PathBuf|ffi::(?:OsString|CString|CStr))\b|\bCow\s*<[^>]*\bstr\b[^>]*>/u;
const RAW_PRIMITIVE_TYPE_RE = /\b(?:bool|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)\b/u;
const RAW_POINTER_RE = /\*(?:const|mut)\s+[A-Za-z_]/u;
const TYPE_ALIAS_RAW_RE = /^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+[A-Z][A-Za-z0-9_]*\s*=\s*([^;]+);/u;
const PUBLIC_SERDE_STRUCT_RE = /^\s*pub\s+struct\s+\w+/u;
const PUBLIC_FIELD_RE = /^\s*pub\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?<type>[^,]+),?/u;

function collectFunctionSignatures(masked) {
  const signatures = [];
  const fnRe =
    /\b(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]+"\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\b/gu;
  let match;
  while ((match = fnRe.exec(masked)) !== null) {
    let end = match.index;
    let parenDepth = 0;
    let angleDepth = 0;
    let seenParen = false;
    for (; end < masked.length; end += 1) {
      const ch = masked[end];
      if (ch === '(') {
        parenDepth += 1;
        seenParen = true;
      } else if (ch === ')') {
        parenDepth = Math.max(0, parenDepth - 1);
      } else if (ch === '<') {
        angleDepth += 1;
      } else if (ch === '>') {
        angleDepth = Math.max(0, angleDepth - 1);
      } else if (seenParen && parenDepth === 0 && angleDepth === 0 && (ch === '{' || ch === ';')) {
        end += 1;
        break;
      }
    }
    signatures.push({ text: masked.slice(match.index, end), index: match.index, line: lineNumberAt(masked, match.index) });
    fnRe.lastIndex = Math.max(fnRe.lastIndex, end);
  }
  return signatures;
}

function normalizedNameTokens(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/gu, '$1_$2')
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .filter(Boolean);
}

function isSuspiciousSerializedFieldName(name) {
  const tokens = normalizedNameTokens(name);
  const lastToken = tokens.at(-1);
  const secondToLastToken = tokens.at(-2);
  return (
    lastToken === 'id' ||
    lastToken === 'ids' ||
    lastToken === 'ref' ||
    lastToken === 'refs' ||
    (secondToLastToken === 'event' && lastToken === 'type') ||
    (secondToLastToken === 'command' && lastToken === 'type')
  );
}

function braceDelta(line) {
  return (line.match(/\{/gu) ?? []).length - (line.match(/\}/gu) ?? []).length;
}

function isTestFile(rel, config) {
  return matchesAnyGlob(rel, config.testFileGlobs);
}

function isRawTypeBoundary(rel, config) {
  return matchesAnyGlob(rel, config.rawTypeBoundaryGlobs);
}

function isRawStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.rawStringOwnerGlobs);
}

function isDomainPrimitiveOwner(rel, config) {
  return matchesAnyGlob(rel, config.domainPrimitiveOwnerGlobs);
}

function isRuntimeStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.runtimeStringOwnerGlobs);
}

function isSerializedDomainOwner(rel, config) {
  return matchesAnyGlob(rel, config.serializedDomainOwnerGlobs);
}

function hasStringLiteral(line) {
  return /"(?:[^"\\]|\\.)*"/u.test(line);
}

function scanRustFile(root, filePath, config) {
  const rel = normalizeRel(root, filePath);
  const violations = [];
  const source = fs.readFileSync(filePath, 'utf8');
  const masked = maskRustCode(source);
  const originalLines = source.split(/\r?\n/u);
  const maskedLines = masked.split(/\r?\n/u);
  const isBoundary = isRawTypeBoundary(rel, config);
  const isStringOwner = isRawStringOwner(rel, config);
  const isPrimitiveOwner = isDomainPrimitiveOwner(rel, config);
  const enforceRuntimeStrings =
    config.enforceRuntimeStringLiterals && !isTestFile(rel, config) && !isRuntimeStringOwner(rel, config);
  const enforceSerializedDomainFields =
    config.enforceSerializedPublicDomainPrimitives && !isTestFile(rel, config) && !isSerializedDomainOwner(rel, config);
  const fileName = path.basename(filePath);
  const badModuleFileNames = new Set(['utils.rs', 'helper.rs', 'helpers.rs', 'common.rs', 'misc.rs', 'stuff.rs', 'shared.rs']);

  if (badModuleFileNames.has(fileName)) {
    addViolation(violations, root, filePath, 1, 'RR-7.4', `Forbidden dumping-ground file name: ${fileName}.`);
  }

  let pendingSerializeDerive = false;
  let pendingSerdeShape = false;
  let trackedSerdeStructDepth = 0;

  maskedLines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const originalLine = originalLines[idx] ?? line;

    if (/^\s*#!?\s*\[\s*(?:allow|expect)\s*\(/u.test(line) || /^\s*#!?\s*\[\s*cfg_attr\s*\([^\]]*\b(?:allow|expect)\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-2.1', 'Lint suppression attribute found.', originalLine);
    }

    if (/rust-rules\s*:\s*(?:ignore|allow|skip|disable)/iu.test(originalLine)) {
      addViolation(violations, root, filePath, lineNo, 'RR-2.2', 'Validator suppression comment found.', originalLine);
    }

    if (/\bunsafe\b/u.test(line)) {
      if (!config.allowUnsafeCode) {
        addViolation(violations, root, filePath, lineNo, 'RR-3.1', 'unsafe keyword found while allowUnsafeCode=false.', originalLine);
      } else if (/\bunsafe\s*\{/u.test(line) && !contextHas(originalLines, idx, 'SAFETY:', 4)) {
        addViolation(violations, root, filePath, lineNo, 'RR-3.2', 'unsafe block lacks nearby // SAFETY: comment.', originalLine);
      } else if (/\bunsafe\s+fn\b/u.test(line) && !contextHas(originalLines, idx, '# Safety', 8)) {
        addViolation(violations, root, filePath, lineNo, 'RR-3.3', 'unsafe fn lacks rustdoc # Safety section.', originalLine);
      }
    }

    if (/\.unwrap\s*\(|\.expect\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-4.1', '.unwrap() or .expect() found.', originalLine);
    }

    if (/\b(?:panic|todo|unimplemented|unreachable)\s*!\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-4.2', 'panic-like macro found.', originalLine);
    }

    if (/\b(?:dbg|println|eprintln)\s*!\s*\(/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-4.3', 'debug/console macro found.', originalLine);
    }

    if (!isBoundary && /\banyhow::Result\b|\bBox\s*<\s*dyn\s+(?:std::error::Error|Error)\b/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-4.4', 'Erased application error type found in non-boundary code.', originalLine);
    }

    if (/\.clone\s*\(/u.test(line) && !contextHas(originalLines, idx, 'CLONE-JUSTIFICATION:', 4)) {
      addViolation(violations, root, filePath, lineNo, 'RR-5.1', '.clone() found without nearby CLONE-JUSTIFICATION.', originalLine);
    }

    if (/\.(?:to_string|to_owned)\s*\(/u.test(line) && !contextHas(originalLines, idx, 'ALLOC-JUSTIFICATION:', 4)) {
      addViolation(violations, root, filePath, lineNo, 'RR-5.2', 'String allocation found without nearby ALLOC-JUSTIFICATION.', originalLine);
    }

    if (/\b[A-Za-z_][A-Za-z0-9_\.]*\s*\[[^\]\n]+\]/u.test(line) && !/\b(?:vec|format|println|assert|assert_eq|assert_ne)!\s*\[/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-5.3', 'Unchecked indexing/slicing found.', originalLine);
    }

    if (/\s+as\s+(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)\b/u.test(line) && !contextHas(originalLines, idx, 'CAST-JUSTIFICATION:', 4)) {
      addViolation(violations, root, filePath, lineNo, 'RR-5.4', 'Numeric cast found without nearby CAST-JUSTIFICATION.', originalLine);
    }

    const typeAliasMatch = line.match(TYPE_ALIAS_RAW_RE);
    if (typeAliasMatch && (RAW_STRING_TYPE_RE.test(typeAliasMatch[1]) || RAW_PRIMITIVE_TYPE_RE.test(typeAliasMatch[1]))) {
      addViolation(violations, root, filePath, lineNo, 'RR-6.5', 'Raw primitive/string type alias found.', originalLine);
    }

    if (/^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\s*\(\s*pub\s+/u.test(line) && (RAW_STRING_TYPE_RE.test(line) || RAW_PRIMITIVE_TYPE_RE.test(line))) {
      addViolation(violations, root, filePath, lineNo, 'RR-6.6', 'Public tuple newtype exposes raw inner field.', originalLine);
    }

    if (/^\s*(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*/u.test(line) && (RAW_STRING_TYPE_RE.test(line) || RAW_PRIMITIVE_TYPE_RE.test(line))) {
      if (/^\s*pub(?:\([^)]*\))?\s+/u.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'RR-6.3', 'Public raw field found.', originalLine);
      } else if (!isBoundary && !contextHas(originalLines, idx, 'BRAND-INVARIANT:', 6)) {
        addViolation(violations, root, filePath, lineNo, 'RR-6.4', 'Private raw field lacks nearby BRAND-INVARIANT documentation.', originalLine);
      }
    }

    if (/^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\s*\([^)]*(?:String|str|bool|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)/u.test(line) && !contextHas(originalLines, idx, 'BRAND-INVARIANT:', 6)) {
      addViolation(violations, root, filePath, lineNo, 'RR-6.4', 'Tuple newtype over raw field lacks BRAND-INVARIANT documentation.', originalLine);
    }

    if (/^\s*use\s+[^;]*::\*\s*;/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-7.1', 'Wildcard import found.', originalLine);
    }

    if (/^\s*pub\s+use\s+[^;]*::\*\s*;/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-7.2', 'Wildcard public re-export found.', originalLine);
    }

    if (/^\s*pub\s+use\s+/u.test(line)) {
      const isFacade = matchesAnyGlob(rel, config.facadeFileGlobs);
      if (config.publicReexportPolicy === 'forbid') {
        addViolation(violations, root, filePath, lineNo, 'RR-7.3', 'pub use is forbidden by this profile.', originalLine);
      } else if (config.publicReexportPolicy === 'facade-only' && !isFacade) {
        addViolation(violations, root, filePath, lineNo, 'RR-7.3', 'pub use outside configured facade file.', originalLine);
      }
    }

    if (/^\s*(?:pub\s+)?mod\s+(?:utils|helper|helpers|common|misc|stuff|shared)\s*;/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-7.4', 'Forbidden dumping-ground module declaration.', originalLine);
    }

    if (/\basync\s+fn\b|\.await\b/u.test(masked) && /\bstd::sync::(?:Mutex|RwLock)\b|\bstd::thread::sleep\b|\bstd::fs::|\bstd::net::/u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-8.1', 'Blocking primitive in async module.', originalLine);
    }

    if (/\bfor\s+[A-Za-z_][A-Za-z0-9_]*\s+in\s+0\s*\.\./u.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'RR-8.2', 'C-style numeric loop found.', originalLine);
    }

    if (enforceRuntimeStrings && hasStringLiteral(originalLine) && !config.runtimeStringLineAllowRegexps.some((pattern) => pattern.test(originalLine))) {
      addViolation(violations, root, filePath, lineNo, 'RR-18.16', 'Runtime Rust source contains an inline string literal.', originalLine);
    }

    if (enforceSerializedDomainFields) {
      if (trackedSerdeStructDepth === 0) {
        if (/^\s*#\[derive\([^#\]]*\b(?:Serialize|Deserialize)\b[^#\]]*\)\]/u.test(line)) {
          pendingSerializeDerive = true;
          return;
        }
        if (/^\s*#\[serde\(/u.test(line)) {
          pendingSerdeShape = true;
          return;
        }
        if (pendingSerializeDerive || pendingSerdeShape) {
          if (/^\s*#\[/u.test(line)) return;
          const shouldTrack = pendingSerializeDerive && pendingSerdeShape && PUBLIC_SERDE_STRUCT_RE.test(line);
          pendingSerializeDerive = false;
          pendingSerdeShape = false;
          if (shouldTrack) trackedSerdeStructDepth = braceDelta(line);
          return;
        }
      } else {
        const match = PUBLIC_FIELD_RE.exec(line);
        if (match && isSuspiciousSerializedFieldName(match.groups.name) && (RAW_STRING_TYPE_RE.test(match.groups.type) || RAW_PRIMITIVE_TYPE_RE.test(match.groups.type))) {
          addViolation(
            violations,
            root,
            filePath,
            lineNo,
            'RR-6.26',
            'Serialized public struct field uses raw id/ref/event/command primitive outside configured owner crates.',
            originalLine
          );
        }
        trackedSerdeStructDepth += braceDelta(line);
      }
    }
  });

  for (const sig of collectFunctionSignatures(masked)) {
    if (isBoundary) continue;
    const originalSigFirstLine = originalLines[sig.line - 1] ?? sig.text;
    if (RAW_POINTER_RE.test(sig.text)) {
      addViolation(violations, root, filePath, sig.line, 'RR-3.4', 'Raw pointer found in function signature.', originalSigFirstLine);
    }
    if (!isStringOwner && RAW_STRING_TYPE_RE.test(sig.text)) {
      addViolation(violations, root, filePath, sig.line, 'RR-6.1', 'Raw string/path type found in function signature.', originalSigFirstLine);
    }
    if (!isPrimitiveOwner && RAW_PRIMITIVE_TYPE_RE.test(sig.text)) {
      addViolation(violations, root, filePath, sig.line, 'RR-6.2', 'Unbranded primitive type found in function signature.', originalSigFirstLine);
    }
  }

  return violations;
}

function scanWorkspaceFiles(root, config, scope) {
  const violations = [];
  const cargoToml = path.join(root, 'Cargo.toml');
  if (!fs.existsSync(cargoToml) || !config.enforceWorkspaceFiles) return violations;

  const required = [
    ['rust-toolchain.toml', 'RR-1.1'],
    ['Cargo.lock', 'RR-1.2'],
    ['clippy.toml', 'RR-1.3'],
    ['deny.toml', 'RR-1.4'],
  ];
  for (const [fileName, ruleId] of required) {
    if (!fs.existsSync(path.join(root, fileName))) {
      addViolation(violations, root, path.join(root, fileName), 1, ruleId, `${fileName} is missing.`);
    }
  }

  const manifestPaths = manifestPathsForScope(root, config, scope);
  for (const manifest of manifestPaths) {
    scanCargoManifest(root, manifest, config, violations);
  }

  return violations;
}

function manifestPathsForScope(root, config, scope) {
  if (scope.mode === 'crate' && scope.manifest) return [scope.manifest];
  if (scope.mode === 'files' || scope.mode === 'diff') {
    return uniqueSorted(scope.files.map((file) => nearestCargoManifest(root, file)).filter(Boolean));
  }
  return findCargoManifests(root, config).filter((manifest) => !normalizeRel(root, manifest).includes('/target/'));
}

function nearestCargoManifest(root, filePath) {
  const rootPath = path.resolve(root);
  let current = path.dirname(path.resolve(filePath));
  while (true) {
    const relative = path.relative(rootPath, current);
    if (relative.startsWith('..') || path.isAbsolute(relative)) return null;
    const manifest = path.join(current, 'Cargo.toml');
    if (fs.existsSync(manifest)) return path.resolve(manifest);
    if (current === rootPath) return null;
    const next = path.dirname(current);
    if (next === current) return null;
    current = next;
  }
}

function scanCargoManifest(root, manifest, config, violations) {
  const cargoText = fs.readFileSync(manifest, 'utf8');
  const packageBlock = cargoText.match(/(?:^|\n)\s*\[package\]([\s\S]*?)(?:\n\s*\[|$)/u);
  const workspacePackageBlock = cargoText.match(/(?:^|\n)\s*\[workspace\.package\]([\s\S]*?)(?:\n\s*\[|$)/u);
  if (
    packageBlock &&
    !/(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(packageBlock[1]) &&
    !/(^|\n)\s*rust-version\s*=\s*"[^"]+"/u.test(workspacePackageBlock?.[1] ?? '')
  ) {
    addViolation(violations, root, manifest, 1, 'RR-1.5', 'Cargo.toml package does not declare rust-version.');
  }

  const lines = cargoText.split(/\r?\n/u);
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    if (/version\s*=\s*"\*"|=\s*"\*"/u.test(line)) {
      addViolation(violations, root, manifest, lineNo, 'RR-9.1', 'Wildcard dependency versions are forbidden.', line);
    }
    if (!config.allowGitDependencies && /\bgit\s*=/u.test(line)) {
      addViolation(violations, root, manifest, lineNo, 'RR-9.2', 'Git dependency found.', line);
    }
    if (!config.allowPathDependencies && /\bpath\s*=/u.test(line)) {
      addViolation(violations, root, manifest, lineNo, 'RR-9.3', 'Path dependency found.', line);
    }
  });

  const buildRs = path.join(path.dirname(manifest), 'build.rs');
  if (!config.allowBuildRs && fs.existsSync(buildRs)) {
    addViolation(violations, root, buildRs, 1, 'RR-7.5', 'build.rs is forbidden by default because it can hide non-deterministic build behavior.');
  }
}

function loadCargoMetadata(root) {
  const result = spawnSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], {
    cwd: root,
    encoding: 'utf8',
    shell: false,
  });
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) return null;
  return JSON.parse(result.stdout);
}

function scanCargoMetadata(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, 'Cargo.toml'))) return violations;
  if (!commandExists('cargo')) return violations;
  const metadata = loadCargoMetadata(root);
  if (!metadata) return violations;
  const workspaceRoot = toPosix(metadata.workspace_root);
  const scopedManifests = new Set(manifestPathsForScope(root, config, scope).map((manifest) => toPosix(manifest)));
  const packageFilter =
    scope.mode === 'crate'
      ? (packageInfo) => packageInfo.name === scope.crateName
      : scope.mode === 'files' || scope.mode === 'diff'
        ? (packageInfo) => scopedManifests.has(toPosix(packageInfo.manifest_path))
        : () => true;
  const packages = (metadata.packages ?? []).filter(packageFilter);

  for (const packageInfo of packages) {
    const blockedForPackage = new Set(config.blockedProtocolDependencies[packageInfo.name] ?? []);
    for (const dependency of packageInfo.dependencies ?? []) {
      if ((dependency.source ?? '').startsWith('git+') && !config.allowedGitDependenciesSet.has(dependency.name)) {
        addViolation(violations, root, packageInfo.manifest_path, 1, 'RR-9.2', 'Git dependency found in cargo metadata.');
      }

      const dependencyPath = dependency.path ?? null;
      if (dependencyPath !== null) {
        const normalizedPath = toPosix(dependencyPath);
        if (!normalizedPath.startsWith(workspaceRoot)) {
          addViolation(violations, root, packageInfo.manifest_path, 1, 'RR-9.4', 'Path dependency points outside the workspace root.');
        }
      }

      if (dependencyPath === null && dependency.req.trim() === '*') {
        addViolation(violations, root, packageInfo.manifest_path, 1, 'RR-9.1', 'Wildcard registry dependency version found.');
      }

      if (blockedForPackage.has(dependency.name)) {
        addViolation(violations, root, packageInfo.manifest_path, 1, 'RR-9.4', `${packageInfo.name} must not depend on ${dependency.name}.`);
      }

      if (config.runtimeCratesSet.has(packageInfo.name) && config.testOnlyCratesSet.has(dependency.name) && dependency.kind !== 'dev') {
        addViolation(violations, root, packageInfo.manifest_path, 1, 'RR-9.4', 'Runtime crate depends on test-only crate outside dev-dependencies.');
      }
    }
  }

  const directReqs = new Map();
  for (const packageInfo of packages) {
    for (const dependency of packageInfo.dependencies ?? []) {
      if ((dependency.path ?? null) !== null || dependency.kind === 'dev' || (dependency.source ?? '').startsWith('git+')) continue;
      if (!directReqs.has(dependency.name)) directReqs.set(dependency.name, new Set());
      directReqs.get(dependency.name).add(dependency.req);
    }
  }
  for (const [dependencyName, reqs] of directReqs) {
    if (reqs.size > 1) {
      addViolation(violations, root, '.', 1, 'RR-9.5', `Direct registry dependency ${dependencyName} uses multiple requirements: ${[...reqs].join(', ')}.`);
    }
  }

  return violations;
}

function runScanner(root, config, scope) {
  const violations = [];
  violations.push(...scanWorkspaceFiles(root, config, scope));
  violations.push(...scanCargoMetadata(root, config, scope));
  for (const filePath of scope.files) {
    violations.push(...scanRustFile(root, filePath, config));
    if (config.failFast && violations.length > 0) break;
  }
  return violations;
}

function commandExists(command) {
  const result = spawnSync(command, ['--version'], { encoding: 'utf8', stdio: 'pipe', shell: false });
  return result.status === 0;
}

function runCommand(root, command, args, ruleId, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    env: { ...process.env, ...env },
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: false,
  });
  if (result.status === 0) return [];
  const output = [result.stdout, result.stderr].filter(Boolean).join('\n').trim();
  return [
    {
      ruleId,
      title: RULES[ruleId].title,
      detail: `${command} ${args.join(' ')} failed with exit code ${result.status ?? 'unknown'}.`,
      file: '.',
      line: 1,
      snippet: RULES[ruleId].snippet,
      source: output.slice(0, 4000),
    },
  ];
}

function shouldRunCargoForScope(scope, config) {
  if (scope.mode === 'all' || scope.mode === 'crate') return true;
  if (scope.mode === 'files') return config.cargoOnFileScope;
  if (scope.mode === 'diff') return config.cargoOnDiffScope;
  return false;
}

function cargoPackageArgs(scope) {
  return scope.mode === 'crate' ? ['--package', scope.crateName] : ['--workspace'];
}

function runCargoGates(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, 'Cargo.toml'))) return violations;
  if (!shouldRunCargoForScope(scope, config)) return violations;

  if (!commandExists('cargo')) {
    violations.push({
      ruleId: 'RR-10.2',
      title: RULES['RR-10.2'].title,
      detail: 'cargo is not installed or not on PATH.',
      file: '.',
      line: 1,
      snippet: 'Install Rust/Cargo and rerun the gate.',
      source: null,
    });
    return violations;
  }

  const packageArgs = cargoPackageArgs(scope);
  violations.push(...runCommand(root, 'cargo', ['fmt', '--all', '--', '--check'], 'RR-10.1'));
  violations.push(
    ...runCommand(
      root,
      'cargo',
      [
        'clippy',
        ...packageArgs,
        '--all-targets',
        '--all-features',
        '--',
        '-D',
        'warnings',
        '-D',
        'clippy::unwrap_used',
        '-D',
        'clippy::expect_used',
        '-D',
        'clippy::panic',
        '-D',
        'clippy::todo',
        '-D',
        'clippy::unimplemented',
        '-D',
        'clippy::dbg_macro',
        '-D',
        'clippy::print_stdout',
        '-D',
        'clippy::print_stderr',
        '-D',
        'clippy::wildcard_imports',
        '-D',
        'clippy::enum_glob_use',
        '-D',
        'clippy::clone_on_ref_ptr',
        '-D',
        'clippy::await_holding_lock',
        '-D',
        'clippy::await_holding_refcell_ref',
      ],
      'RR-10.2'
    )
  );

  const testArgs = ['test', ...packageArgs, '--all-features'];
  if (config.cargoTestThreads !== null) {
    testArgs.push('--', `--test-threads=${config.cargoTestThreads}`);
  }
  violations.push(...runCommand(root, 'cargo', testArgs, 'RR-10.3'));

  if (config.runCargoDoc) {
    violations.push(...runCommand(root, 'cargo', ['doc', ...packageArgs, '--all-features', '--no-deps'], 'RR-10.4', {
      RUSTDOCFLAGS: '-D warnings -D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls -D missing_docs',
    }));
  }

  if (config.requireCargoDeny) {
    if (!commandExists('cargo-deny')) {
      violations.push({
        ruleId: 'RR-11.2',
        title: RULES['RR-11.2'].title,
        detail: 'cargo-deny is required but not installed or not on PATH.',
        file: '.',
        line: 1,
        snippet: RULES['RR-11.2'].snippet,
        source: null,
      });
    } else {
      violations.push(...runCommand(root, 'cargo', ['deny', 'check'], 'RR-11.1'));
    }
  }

  if (config.requireCargoAudit) {
    if (!commandExists('cargo-audit')) {
      violations.push({
        ruleId: 'RR-11.3',
        title: RULES['RR-11.3'].title,
        detail: 'cargo-audit is enabled but not installed or not on PATH.',
        file: '.',
        line: 1,
        snippet: RULES['RR-11.3'].snippet,
        source: null,
      });
    } else {
      violations.push(...runCommand(root, 'cargo', ['audit'], 'RR-11.3'));
    }
  }

  return violations;
}

function printHumanReport(report) {
  if (report.violations.length === 0) {
    console.log(`Ocentra Enforcer ${report.command} passed for ${report.scope.files.length} file(s).`);
    return;
  }

  console.error(`Ocentra Enforcer ${report.command} failed with ${report.violations.length} violation${report.violations.length === 1 ? '' : 's'}.`);
  console.error(`Profile: ${report.profileName}`);
  console.error(`Scope: ${describeScope(report.scope)}`);
  console.error('');
  for (const violation of report.violations) {
    console.error(`${violation.file}:${violation.line}: ${violation.ruleId} ${violation.title}`);
    console.error(`  Reason: ${violation.detail}`);
    console.error(`  Rule: ${ruleDocFor(violation.ruleId)}`);
    console.error(`  Fix: ${violation.snippet}`);
    if (violation.source) {
      for (const line of String(violation.source).split(/\r?\n/u).slice(0, 12)) console.error(`  > ${line}`);
    }
    console.error('');
  }
}

function describeScope(scope) {
  if (scope.mode === 'crate') return `crate ${scope.crateName}`;
  if (scope.mode === 'diff') return `diff ${scope.base}..${scope.head}`;
  if (scope.mode === 'files') return 'explicit files';
  return 'workspace';
}

function explainRule(ruleId) {
  const normalized = ruleId?.toUpperCase();
  const rule = RULES[normalized];
  if (!rule) throw new Error(`Unknown rule ID: ${ruleId}`);
  return { ruleId: normalized, ...rule, anchor: ruleDocFor(normalized) };
}

function ruleDocFor(ruleId) {
  if (cachedRuleDocs === null) {
    cachedRuleDocs = new Map();
    if (fs.existsSync(RULE_REGISTRY_PATH)) {
      const registry = decodeRuleRegistry(JSON.parse(fs.readFileSync(RULE_REGISTRY_PATH, 'utf8')));
      for (const entry of registry.rules ?? []) cachedRuleDocs.set(entry.id, entry.doc);
    }
  }
  return cachedRuleDocs.get(ruleId) ?? `docs/RustRules.md#${ruleId.toLowerCase().replace('.', '')}`;
}

function doctor(root, config, scope) {
  const checks = [
    { name: 'root', ok: fs.existsSync(root), detail: root },
    { name: 'config schema', ok: config.schemaVersion >= 1, detail: `schemaVersion=${config.schemaVersion}` },
    { name: 'cargo', ok: commandExists('cargo'), detail: 'required for cargo gates and metadata dependency checks' },
    { name: 'git', ok: commandExists('git'), detail: 'required for diff scopes' },
    {
      name: 'cargo-deny',
      ok: !config.requireCargoDeny || commandExists('cargo-deny'),
      detail: config.requireCargoDeny ? 'required when requireCargoDeny=true' : 'not required by this profile',
    },
    { name: 'scope files', ok: scope.files.length > 0, detail: `${scope.files.length} Rust file(s) selected` },
  ];
  return {
    ok: checks.every((check) => check.ok),
    command: 'doctor',
    root,
    profileName: config.profileName,
    scope,
    checks,
    violations: [],
  };
}

function printDoctor(report) {
  for (const check of report.checks) {
    console.log(`${check.ok ? 'PASS' : 'FAIL'} ${check.name}: ${check.detail}`);
  }
}

export function runRustRules(options = {}) {
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({ ...DEFAULT_CONFIG, ...(options.config ?? loadConfig(root, options.configPath)) });
  const scope = resolveScope(root, config, options.scope ?? { mode: 'all' });
  const command = options.command ?? 'scan';
  const scannerViolations = runScanner(root, config, scope);
  const cargoViolations = options.scanOnly || command === 'scan' ? [] : runCargoGates(root, config, scope);
  const violations = [...scannerViolations, ...cargoViolations];
  return {
    ok: violations.length === 0,
    command,
    violations,
    root,
    profileName: config.profileName,
    scanOnly: Boolean(options.scanOnly || command === 'scan'),
    scope: { ...scope, files: scope.files.map((file) => normalizeRel(root, file)) },
  };
}

function isMainModule() {
  return process.argv[1] && path.resolve(process.argv[1]) === SCRIPT_PATH;
}

if (isMainModule()) {
  try {
    const args = parseArgs(process.argv);
    if (args.help) {
      console.log(usage());
      process.exit(0);
    }

    if (args.command === 'init') {
      const report = createInitReport(args);
      if (!report.dryRun) applyInitReport(report);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printInitReport(report);
      process.exit(0);
    }

    const root = path.resolve(args.root);
    const config = loadConfig(root, args.configPath);
    const scope = resolveScope(root, config, args.scope);

    if (args.command === 'explain') {
      const report = explainRule(args.explainRuleId);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else {
        console.log(`${report.ruleId} ${report.title}`);
        console.log(`Rule: ${report.anchor}`);
        console.log(`Fix: ${report.snippet}`);
      }
      process.exit(0);
    }

    if (args.command === 'doctor') {
      const report = doctor(root, config, scope);
      if (args.json) console.log(JSON.stringify(report, null, 2));
      else printDoctor(report);
      process.exit(report.ok ? 0 : 1);
    }

    const report = runRustRules({
      root,
      config,
      scope,
      command: args.command,
      scanOnly: args.scanOnly,
    });
    if (args.json) console.log(JSON.stringify(report, null, 2));
    else printHumanReport(report);
    process.exit(report.ok ? 0 : 1);
  } catch (error) {
    console.error(`Ocentra Enforcer internal error: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(2);
  }
}
