#!/usr/bin/env node
import { runCoordinationCli } from "../src/coordination/runner.mjs";

await runCoordinationCli(process.argv.slice(2));
