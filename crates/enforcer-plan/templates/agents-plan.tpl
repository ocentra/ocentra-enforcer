<!-- agents-forest-tier: plan -->
# AGENTS.md ({{plan_name}})

<!-- agents-read-first -->
> READ ME FIRST. This is the PLAN tier of the AGENTS.md decision forest
> for plan `{{plan_name}}`. Read this file after the global and project
> tiers. This is the LAST routing tier before the plan's resume-state.
> Budget: stay under {{budget_lines}} lines / {{budget_bytes}} bytes for
> this tier.
<!-- /agents-read-first -->

<!-- agents-next-tier -->
> NEXT: {{next_tier_path}}
<!-- /agents-next-tier -->

<!-- agents-decision-tree -->
> DECISION TREE
> - if resuming work -> read {{resume_anchor}}
> - if starting fresh -> read {{next_tier_path}}
> - if blocked -> read {{resume_anchor}}
> LEAF -> {{resume_anchor}}
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

Per-plan router for `docs/plans/{{plan_name}}/`.
