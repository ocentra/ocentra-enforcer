<!-- agents-forest-tier: global -->
# AGENTS.md (global)

<!-- agents-read-first -->
> READ ME FIRST. This is the GLOBAL tier of the AGENTS.md decision forest.
> Read this file before any project or plan doc. Budget: stay under
> 40 lines / 2048 bytes for this tier.
<!-- /agents-read-first -->

<!-- agents-next-tier -->
> NEXT: pass/project/AGENTS.md
<!-- /agents-next-tier -->

<!-- agents-decision-tree -->
> DECISION TREE
> - if resuming work -> read pass/project/AGENTS.md
> - if starting fresh -> read pass/project/AGENTS.md
> - if blocked -> read pass/project/AGENTS.md
> LEAF -> docs/plans/enforcer-selfhost-plan/RESUME_STATE.md
<!-- /agents-decision-tree -->

<!-- agents-transitional-intent -->
> TRANSITIONAL-TO-TYPED: this file is a transitional prose surface. It is
> designed to be dropped for a typed system/db/schema later. The routing
> chain and decision tree are modeled as data (tier -> next-pointer ->
> decision-node -> resume-anchor) so the same structure can be served from
> a typed store and rendered in the Tauri desktop UI for humans. Do NOT
> hard-couple any validator to this prose surviving forever: parse the
> structured markers above, not free text, so the backing store can swap
> under a stable contract.
<!-- /agents-transitional-intent -->

Machine/workspace root router: dev-machine.
