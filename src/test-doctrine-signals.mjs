/*
 * Declarative evidence tables for test-doctrine detection. Each category maps
 * to filename patterns and/or manifest-text substrings that count as evidence
 * a project already has that kind of testing wired up. Cheap, signal-based —
 * mirrors the regex-line-scan philosophy of the generic language scanners.
 */

const MANIFEST_NAME_RE = /(^|\/)(package\.json|pyproject\.toml|requirements.*\.txt|Cargo\.toml|Pipfile)$/i;

// Candidate files to content-scan for the harder-to-name-detect categories below.
// Bounded to test-shaped files only — never a whole-repo content grep.
const TEST_FILE_RE = /\.(test|spec)\.[jt]sx?$|(^|\/)test_[^/]+\.py$|(^|\/)[^/]+_test\.py$/i;

const CATEGORY_SIGNALS = {
  unit: {
    filenames: [
      /\.(test|spec)\.[jt]sx?$/i,
      /(^|\/)test_[^/]+\.py$/i,
      /(^|\/)[^/]+_test\.py$/i,
      /(^|\/)tests?\/.+\.(py|[jt]sx?)$/i,
    ],
    manifestText: [],
  },
  integration: {
    filenames: [
      /(^|\/)tests?\/integration\//i,
      /(^|\/)integration[-_]?tests?\//i,
      /\.(integration)\.(test|spec)\.[jt]sx?$/i,
    ],
    manifestText: [],
  },
  e2e: {
    filenames: [
      /(^|\/)(e2e|end-to-end)\//i,
      /playwright\.config\.[jt]s$/i,
      /cypress\.config\.[jt]s$/i,
      /(^|\/)cypress\//i,
    ],
    manifestText: [/@playwright\/test/i, /"cypress"/i, /puppeteer/i, /selenium-webdriver/i],
  },
  contract: {
    filenames: [
      /(^|\/)pacts?\//i,
      /\bcontract\b.*\.(test|spec)\.[jt]sx?$/i,
      /(^|\/)test_[^/]*contract[^/]*\.py$/i,
      /(^|\/)[^/]*contract[^/]*_test\.py$/i,
      /(^|\/)contracts?[-_]?tests?\//i,
      /schemathesis/i,
    ],
    manifestText: [/@pact-foundation\/pact/i, /pact-python/i, /schemathesis/i, /openapi-diff/i],
  },
  mutation: {
    filenames: [
      /stryker\.conf\.[jmc]?[jt]s$/i,
      /mutmut\.ini$/i,
      /cosmic-ray\.toml$/i,
      /\.cargo-mutants\.toml$/i,
    ],
    manifestText: [/@stryker-mutator\/core/i, /\bmutmut\b/i, /cargo-mutants/i, /cosmic-ray/i],
  },
  propertyFuzzing: {
    filenames: [/(^|\/)\.hypothesis\//i],
    manifestText: [/fast-check/i, /\bhypothesis\b/i, /proptest/i, /quickcheck/i, /schemathesis/i, /atheris/i],
  },
  security: {
    filenames: [
      /(^|\/)\.semgrep\.ya?ml$/i,
      /(^|\/)\.github\/codeql/i,
      /gitleaks\.toml$/i,
      /(^|\/)\.zap\//i,
      /(^|\/)\.bandit$/i,
    ],
    manifestText: [/\bbandit\b/i, /\bsemgrep\b/i, /pip-audit/i, /\bsafety\b/i],
  },
  snapshot: {
    filenames: [/(^|\/)__snapshots__\//i, /\.snap$/i],
    manifestText: [],
  },
  loadPerformance: {
    filenames: [/(^|\/)k6\//i, /artillery\.(ya?ml|json)$/i, /locustfile\.py$/i, /\.gatling\./i],
    manifestText: [/\bk6\b/i, /\bartillery\b/i, /\blocust\b/i],
  },
  coverageTooling: {
    filenames: [/\.nycrc/i, /(^|\/)c8\.config/i, /\.coveragerc$/i],
    manifestText: [/pytest-cov/i, /"c8"/i, /"nyc"/i, /coverage\[toml\]/i],
  },
  concurrencyRaceTests: {
    filenames: [/concurren(cy|t)/i, /race[-_]?condition/i],
    manifestText: [],
    content: {
      filePattern: TEST_FILE_RE,
      textPatterns: [/asyncio\.gather/i, /Promise\.all/i, /concurrent\.futures/i, /ThreadPoolExecutor/i, /race[- ]?condition/i, /retry[- ]?storm/i],
    },
  },
  idempotencyReplayTests: {
    filenames: [/idempoten(cy|t)/i, /\breplay\b/i],
    manifestText: [],
    content: {
      filePattern: TEST_FILE_RE,
      textPatterns: [/idempoten/i, /\breplay(ed|ing)?\b/i, /duplicate[- _]?request/i],
    },
  },
  rollbackCompensationTests: {
    filenames: [/rollback/i, /compensat/i],
    manifestText: [],
    content: {
      filePattern: TEST_FILE_RE,
      textPatterns: [/rollback/i, /compensat(e|ion|ing)/i],
    },
  },
  timeClockTests: {
    filenames: [],
    manifestText: [/freezegun/i, /time[-_]machine/i],
    content: {
      filePattern: TEST_FILE_RE,
      textPatterns: [/freeze_time/i, /useFakeTimers/i, /travel_to/i, /clock[- _]?skew/i, /mock.*(datetime|clock)/i],
    },
  },
  economicInvariantTests: {
    filenames: [],
    manifestText: [],
    content: {
      filePattern: TEST_FILE_RE,
      textPatterns: [/\binvariant\b/i, /balance.*(unchanged|preserved|conserv)/i, /double[- _]?(spend|charge|book)/i],
    },
  },
  killSwitchTests: {
    filenames: [/kill[-_ ]?switch/i, /circuit[-_ ]?breaker/i, /emergency[-_ ]?(stop|disable)/i],
    manifestText: [],
    content: {
      filePattern: TEST_FILE_RE,
      textPatterns: [/kill[- _]?switch/i, /circuit[- _]?breaker/i, /emergency[- _]?(disable|stop|halt)/i],
    },
  },
};

export { CATEGORY_SIGNALS, MANIFEST_NAME_RE };
