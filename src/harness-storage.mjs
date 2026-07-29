/** Determines whether a stored harness run matches a query. */
export function matchesRunQuery(run, args) {
  if (args.runId && run.runId !== args.runId) return false;
  if (args.status && run.status !== args.status) return false;
  if (args.tool && run.tool !== args.tool) return false;
  if (args.crateName && run.crateName !== args.crateName) return false;
  if (args.packageName && run.packageName !== args.packageName) return false;
  if (args.domain && run.domain !== args.domain) return false;
  if (args.tag && !(run.tags ?? []).includes(args.tag)) return false;
  return true;
}
