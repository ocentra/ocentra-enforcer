<!-- agents-forest-tier: project -->
# AGENTS.md (project)

<!-- agents-read-first -->
> READ ME FIRST. This is the PROJECT tier of the AGENTS.md decision forest
> for repo `ocentra-enforcer`. Read this file after the global tier and
> before any per-plan tier. Budget: stay under 40 lines / 2048 bytes for
> this tier.
<!-- /agents-read-first -->

<!-- agents-next-tier -->
> NEXT: pass/plan/AGENTS.md
<!-- /agents-next-tier -->

<!-- agents-decision-tree -->
> DECISION TREE
> - if resuming work -> read pass/plan/AGENTS.md
> - if starting fresh -> read pass/plan/AGENTS.md
> - if blocked -> read pass/plan/AGENTS.md
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

Repo root router for `ocentra-enforcer`.
