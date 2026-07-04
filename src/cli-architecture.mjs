import process from "node:process";
import { collectFilesFromManifest } from "../scripts/rust-rules-scan-core-args-files.mjs";

function assignNext(key) {
  return (args, tokens, index) => {
    args[key] = tokens[index + 1];
    return index + 1;
  };
}

function setScope(scopeName) {
  return (args, _tokens, index) => {
    args.scopeName = scopeName;
    return index;
  };
}

function setJsonFlag(args, _tokens, index) {
  args.json = true;
  return index;
}

function collectFiles(args, tokens, index) {
  args.scopeName = "files";
  let nextIndex = index;
  while (tokens[nextIndex + 1] && !tokens[nextIndex + 1].startsWith("-")) {
    args.files.push(tokens[++nextIndex]);
  }
  return nextIndex;
}

function collectFilesFromScopeManifest(args, tokens, index) {
  args.scopeName = "files";
  args.fileManifests.push(tokens[index + 1]);
  return index + 1;
}

function collectFilesFromInlineManifest(args, _tokens, index, token) {
  args.scopeName = "files";
  args.fileManifests.push(token.slice("--files-from=".length));
  return index;
}

const TOKEN_HANDLERS = {
  "--root": assignNext("root"),
  "--config": assignNext("configPath"),
  "--profile": assignNext("profile"),
  "--language": assignNext("language"),
  "--scope": assignNext("scopeName"),
  "--base": assignNext("base"),
  "--head": assignNext("head"),
  "--all": setScope("all"),
  "--workspace": setScope("all"),
  "--json": setJsonFlag,
  "--files": collectFiles,
  "--files-from": collectFilesFromScopeManifest,
};

export function parseArchitectureCheckTokens(tokens) {
  const args = {
    root: process.cwd(),
    language: "rust",
    scopeName: "files",
    files: [],
    base: null,
    head: null,
    configPath: null,
    profile: null,
    json: false,
    fileManifests: [],
  };
  for (let index = 1; index < tokens.length; index += 1) {
    const token = tokens[index];
    const handler = TOKEN_HANDLERS[token];
    if (handler) {
      index = handler(args, tokens, index);
    } else if (token.startsWith("--files-from=")) {
      index = collectFilesFromInlineManifest(args, tokens, index, token);
    } else if (token.startsWith("-")) {
      throw new Error(`Unknown architecture argument: ${token}`);
    } else {
      args.files.push(token);
    }
  }
  if (args.language !== "rust") {
    throw new Error("architecture check currently supports --language rust");
  }
  for (const manifestPath of args.fileManifests) {
    args.files.push(...collectFilesFromManifest(manifestPath, args.root));
  }
  return {
    json: args.json,
    root: args.root,
    configPath: args.configPath,
    profile: args.profile,
    rawScope: resolveArchitectureScope(args),
  };
}

function resolveArchitectureScope(args) {
  if (args.scopeName === "all" || args.scopeName === "workspace") {
    return { mode: "all" };
  }
  if (args.scopeName === "diff") {
    if (!args.base || !args.head) {
      throw new Error("architecture diff scope requires --base <sha> --head <sha>");
    }
    return { mode: "diff", base: args.base, head: args.head };
  }
  return { mode: "files", files: args.files };
}
