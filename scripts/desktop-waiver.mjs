import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { upsertPackagedWaiverRegistry } from "../src/packaged-waivers.mjs";

const args = Object.fromEntries(process.argv.slice(2).filter((_, index, values) => index % 2 === 0).map((key, index) => [key, process.argv.slice(2)[index * 2 + 1]]));
const required = ["--root", "--path", "--rule", "--owner", "--reason"];
for (const key of required) if (!args[key]) throw new Error(`Missing ${key}.`);
const rules = JSON.parse(fs.readFileSync(new URL("../rules/rules.json", import.meta.url), "utf8")).rules;
const waiver = upsertPackagedWaiverRegistry(path.join(path.resolve(args["--root"]), ".enforce", "waivers.json"), rules, { path: args["--path"], ruleId: args["--rule"], owner: args["--owner"], reason: args["--reason"], expires: args["--expires"] || null });
process.stdout.write(`${JSON.stringify({ waiver })}\n`);
