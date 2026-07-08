/*
 * Heuristic "project nature" detection — drives which missing-test categories
 * are actually worth suggesting. A project with no frontend shouldn't be told
 * it's missing Playwright; a project with no billing code shouldn't be told
 * it needs economic-invariant tests.
 */

const LANGUAGE_EXTENSIONS = {
  python: /\.py$/i,
  typescript: /\.tsx?$/i,
  javascript: /\.jsx?$/i,
  rust: /\.rs$/i,
  go: /\.go$/i,
};

const WEB_API_MANIFEST_RE = /fastapi|flask|django|express|@nestjs|koa|actix-web|axum/i;
const FRONTEND_MANIFEST_RE = /"react"|"vue"|"@angular\/core"|"svelte"/i;
const ASYNC_WORKER_MANIFEST_RE = /boto3|celery|bullmq|kafka-python|\bpika\b|aiokafka/i;
const ASYNC_WORKER_FILENAME_RE = /(^|\/)workers?\//i;

const MONEY_FILENAME_RE = /billing|payment|invoice|stripe|wallet|credit|balance|pricing|checkout|subscription/i;
const CLIENT_FILENAME_RE = /(^|\/)[^/]*client\.(py|[jt]sx?)$/i;

function countLanguages(relPaths) {
  const counts = {};
  for (const [lang, re] of Object.entries(LANGUAGE_EXTENSIONS)) {
    const count = relPaths.filter((p) => re.test(p)).length;
    if (count > 0) counts[lang] = count;
  }
  return counts;
}

function detectNature(relPaths, manifestText) {
  const languages = countLanguages(relPaths);
  const hasOpenApiSpec = relPaths.some((p) => /openapi\.(json|ya?ml)$|swagger\.(json|ya?ml)$/i.test(p));
  const isWebApi = WEB_API_MANIFEST_RE.test(manifestText) || hasOpenApiSpec;
  const hasFrontendUi = FRONTEND_MANIFEST_RE.test(manifestText);
  const hasAsyncWorkers = ASYNC_WORKER_MANIFEST_RE.test(manifestText)
    || relPaths.some((p) => ASYNC_WORKER_FILENAME_RE.test(p));
  const moneyFiles = relPaths.filter((p) => MONEY_FILENAME_RE.test(p));
  const clientFiles = relPaths.filter((p) => CLIENT_FILENAME_RE.test(p));

  return {
    languages,
    isWebApi,
    hasOpenApiSpec,
    hasFrontendUi,
    hasAsyncWorkers,
    hasMoneyCriticalSurface: moneyFiles.length > 0,
    moneyCriticalFiles: moneyFiles.slice(0, 10),
    hasMultiServiceBoundary: clientFiles.length > 0,
    multiServiceClientFiles: clientFiles.slice(0, 10),
  };
}

export { detectNature };
