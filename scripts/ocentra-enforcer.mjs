#!/usr/bin/env node
/*
 * Canonical Ocentra Enforcer CLI entrypoint.
 * rust-rules remains a compatibility alias while consumers are rewired.
 */
import process from "node:process";
import { main } from "../src/cli-main.mjs";

const exitCode = await main(process.argv);
process.exit(exitCode);
