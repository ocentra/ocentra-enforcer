export function appendScopeArgs(cliArgs, args) {
  const scope = inferScope(args);
  for (const token of scopeArgs(scope, args)) {
    cliArgs.push(token);
  }
}

function inferScope(args) {
  if (args.scope) return args.scope;
  if (Array.isArray(args.files) && args.files.length > 0) return "files";
  if (args.crateName) return "crate";
  return args.base || args.head ? "diff" : "workspace";
}

function scopeArgs(scope, args) {
  switch (scope) {
    case "files":
      return fileScopeArgs(args.files);
    case "crate":
      return crateScopeArgs(args.crateName);
    case "diff":
      return diffScopeArgs(args.base, args.head);
    default:
      return ["--workspace"];
  }
}

function fileScopeArgs(files) {
  if (!Array.isArray(files) || files.length === 0) {
    throw new Error("files scope requires files.");
  }
  return ["--files", ...files];
}

function crateScopeArgs(crateName) {
  if (!crateName) {
    throw new Error("crate scope requires crateName.");
  }
  return ["--crate", crateName];
}

function diffScopeArgs(base, head) {
  if (!base || !head) {
    throw new Error("diff scope requires base and head.");
  }
  return ["--base", base, "--head", head];
}

