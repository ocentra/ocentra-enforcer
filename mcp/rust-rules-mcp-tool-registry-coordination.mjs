import {
  coordinationActionInputSchema,
  coordinationInputSchema,
} from "./rust-rules-mcp-helpers.mjs";

export const COORDINATION_TOOLS = [
  {
    name: "ocentra_enforcer_coordination_init",
    description: "Initialize generic external coordination state for a hub/lane.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_health",
    description:
      "Return compact generic hub/lane/mail/worktree coordination health and write-safety decisions.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_presence",
    description:
      "Return compact PC/project/worktree/lane/thread/claim presence matrix for the coordination hub.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_index",
    description:
      "Rebuild disposable coordination read indexes and JSON views from canonical streams.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_streams",
    description:
      "Return stream manifest with event counts, byte lengths, seq ranges, and tail hashes.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_sync",
    description:
      "Sync coordination streams from a local or HTTP peer using manifest plus suffix transfer.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_peer",
    description:
      "Manage and inspect coordination peers: add, remove, list, health, status, or sync.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_ensure",
    description:
      "Ensure the background coordination peer daemon is running for this state root.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_compact",
    description:
      "Compact hot streams into immutable archive segments and rebuild read indexes.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_notify",
    description: "Return wake/notification requests for a coordination lane.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_mail",
    description: "Aggregate mail helper for inbox, send, and ack actions.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_status",
    description: "Return materialized generic coordination state.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_inbox",
    description: "Return unread or all messages for a lane.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_claim",
    description:
      "Claim exact paths for a lane. Use ocentra_enforcer_coordination_release to release paths.",
    inputSchema: coordinationActionInputSchema(
      "claim",
      "Optional dedicated-tool action marker. action=\"release\" is invalid here.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_release",
    description:
      "Release exact paths for a lane. Use ocentra_enforcer_coordination_claim to claim paths.",
    inputSchema: coordinationActionInputSchema(
      "release",
      "Optional dedicated-tool action marker. action=\"claim\" is invalid here.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_closeout",
    description:
      "Release and stale-repair all claims for the selected lane/thread scope, rebuild indexes, and fail if any claims remain.",
    inputSchema: coordinationActionInputSchema(
      "closeout",
      "Optional dedicated-tool action marker.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_repair",
    description:
      "Dry-run or apply safe coordination stream compatibility repairs, such as legacy-hash repair for context-bearing events.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_guard",
    description: "Check whether a lane may write the provided exact paths.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_report",
    description: "Append a generic coordination lifecycle report.",
    inputSchema: coordinationActionInputSchema(
      "report",
      "Optional dedicated-tool action marker.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_message",
    description: "Send generic coordination mail to a lane/address.",
    inputSchema: coordinationActionInputSchema(
      ["message", "send"],
      "Optional dedicated-tool action marker. Use coordination_mail for aggregate mail actions.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_workers",
    description: "Return compact worker status.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_tasks",
    description: "Return active task status.",
    inputSchema: coordinationInputSchema(),
  },
];
