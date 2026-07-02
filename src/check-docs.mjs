import fs from "node:fs";
import path from "node:path";
import { lineNumberAt, normalizeRel } from "./path-utils.mjs";
import {
  collectSourceFiles,
  escapeRegExp,
  finding,
  loadRegistryRules,
  markdownAnchor,
  markdownAnchors,
  resolvePackRoot,
} from "../scripts/check-source-core-helpers.mjs";

export function collectDocsCompletenessFindings(root, args = {}) {
  const packRoot = resolvePackRoot(root, args);
  const findings = [];
  const rules = loadRegistryRules(packRoot);
  const registryPath = path.join(packRoot, "rules", "rules.json");
  const requiredHeadings = ["Covered Rules", "Fails", "Passes", "Fix Recipe", "Validator"];
  for (const file of collectSourceFiles(path.join(packRoot, "rules"), [".md"])) {
    if (path.basename(file) === "INDEX.md") continue;
    const text = fs.readFileSync(file, "utf8");
    const anchors = new Set(markdownAnchors(text));
    const missing = requiredHeadings.filter(
      (heading) => !anchors.has(markdownAnchor(heading)),
    );
    if (missing.length > 0) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.1",
          `${normalizeRel(packRoot, file)} is missing rule doc sections: ${missing.join(", ")}`,
          null,
        ),
      );
    }
    const rel = normalizeRel(packRoot, file);
    const docRules = rules.filter((rule) => String(rule.doc ?? "").split("#")[0] === rel);
    const sourceRules = docRules.filter((rule) =>
      ["rust", "typescript", "python"].includes(String(rule.language ?? ""))
      && ["source", "domain", "imports-modules"].includes(String(rule.family ?? "")),
    );
    if (sourceRules.length > 0 && !hasFailAndPassCodeBlocks(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.2",
          `${rel} covers source rules but lacks both fail and pass fenced code examples`,
          null,
        ),
      );
    }
    const immutableRules = docRules.filter((rule) => rule.lockLevel === "immutable");
    if (immutableRules.length > 0 && /\bshould\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          firstMatchingLine(text, /\bshould\b/iu),
          "DOCENF-1.6",
          `${rel} documents immutable rules with advisory "should" language`,
          null,
        ),
      );
    }
    if (/\brust-rules\b/iu.test(text) && !/compatibility alias/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          firstMatchingLine(text, /\brust-rules\b/iu),
          "DOCENF-1.7",
          `${rel} refers to rust-rules without saying it is a compatibility alias`,
          null,
        ),
      );
    }
    findings.push(...collectTaggedCodeBlockFindings(root, file, text));
    if (/\b(?:rust-only|rust only|typescript\/python later|python\/typescript later)\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          firstMatchingLine(text, /\b(?:rust-only|rust only|typescript\/python later|python\/typescript later)\b/iu),
          "DOCENF-1.8",
          `${rel} contains stale single-language positioning despite multi-language rules`,
          null,
        ),
      );
    }
    const advisoryRules = docRules.filter((rule) => rule.lockLevel === "advisory");
    if (advisoryRules.length > 0 && !/\b(?:promote|profile|failOn|severity|warning|error)\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.9",
          `${rel} covers advisory rules but does not explain profile promotion or severity handling`,
          null,
        ),
      );
    }
    const reviewOrProofRules = docRules.filter((rule) =>
      ["review", "proof"].includes(String(rule.validator ?? ""))
      || String(rule.family ?? "").includes("proof"),
    );
    if (reviewOrProofRules.length > 0 && !/\b(?:proof|checklist|review evidence|evidence)\b/iu.test(text)) {
      findings.push(
        finding(
          root,
          file,
          1,
          "DOCENF-1.10",
          `${rel} covers review/proof rules but does not name the expected evidence`,
          null,
        ),
      );
    }
  }
  for (const rule of rules) {
    const [, anchor = ""] = String(rule.doc ?? "").split("#");
    if (anchor && anchor !== markdownAnchor(anchor)) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "DOCENF-1.5",
          `${rule.id} uses unstable doc anchor #${anchor}; use #${markdownAnchor(anchor)}`,
          rule.doc,
        ),
      );
    }
    if (String(rule.snippet ?? "").length > 240) {
      findings.push(
        finding(
          root,
          registryPath,
          1,
          "DOCENF-1.4",
          `${rule.id} snippet is longer than 240 characters`,
          rule.snippet,
        ),
      );
    }
  }
  return findings;
}

function collectTaggedCodeBlockFindings(root, file, markdown) {
  const findings = [];
  const blockPattern = /```([A-Za-z0-9_-]+)\s*\n([\s\S]*?)```/gu;
  for (const match of markdown.matchAll(blockPattern)) {
    const language = String(match[1] ?? "").toLowerCase();
    const code = String(match[2] ?? "");
    const line = lineNumberAt(markdown, match.index ?? 0);
    if (language === "json") {
      try {
        JSON.parse(code);
      } catch (error) {
        findings.push(
          finding(
            root,
            file,
            line,
            "DOCENF-1.3",
            `JSON code block is not parseable: ${error instanceof Error ? error.message : String(error)}`,
            null,
          ),
        );
      }
    }
    if (["js", "javascript", "ts", "typescript", "tsx", "rust", "rs", "python", "py"].includes(language) && !delimitersBalanced(code)) {
      findings.push(
        finding(
          root,
          file,
          line,
          "DOCENF-1.3",
          `${language} code block has unbalanced delimiters`,
          null,
        ),
      );
    }
  }
  return findings;
}

function delimitersBalanced(code) {
  const pairs = new Map([["}", "{"], [")", "("], ["]", "["]]);
  const stack = [];
  let quote = null;
  for (let index = 0; index < code.length; index += 1) {
    const char = code[index];
    const previous = code[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") quote = null;
      continue;
    }
    if (char === "\"" || char === "'" || char === "`") {
      quote = char;
      continue;
    }
    if (["{", "(", "["].includes(char)) stack.push(char);
    if (pairs.has(char) && stack.pop() !== pairs.get(char)) return false;
  }
  return stack.length === 0 && quote === null;
}

function hasFailAndPassCodeBlocks(markdown) {
  const failSection = markdownSection(markdown, "Fails");
  const passSection = markdownSection(markdown, "Passes");
  const codeBlock = /```(?:rust|rs|typescript|ts|tsx|python|py|js|javascript)?\s*[\s\S]*?```/iu;
  return (codeBlock.test(failSection) && codeBlock.test(passSection))
    || (/Fails:\s*\n\s*```[\s\S]*?```/iu.test(markdown)
      && /Passes:\s*\n\s*```[\s\S]*?```/iu.test(markdown));
}

function markdownSection(markdown, heading) {
  const lines = markdown.split(/\r?\n/u);
  const start = lines.findIndex((line) => new RegExp(`^##+\\s+${escapeRegExp(heading)}\\s*$`, "iu").test(line));
  if (start < 0) return "";
  const end = lines.findIndex((line, index) => index > start && /^##+\s+/u.test(line));
  return lines.slice(start + 1, end < 0 ? lines.length : end).join("\n");
}

function firstMatchingLine(text, pattern) {
  const lines = text.split(/\r?\n/u);
  const index = lines.findIndex((line) => pattern.test(line));
  return index < 0 ? 1 : index + 1;
}
