# RM04 — Configuration, Rule, Route, and Proof Oracle

<!-- agent-capsule -->
```yaml
id: RM04
owns: "read-only config/rule/route/proof rows"
deps: "RM01"
tier: P1
owner: "Luna read-only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

`rust-rules.config.json`, profiles, rules, routing, and proof records have MJS and native representations.

## Where We Want To Be

Every row has golden valid/invalid config, route result, rule/fixture/doc link, and proof-record semantics.

## Acceptance And Proof

One child audits one family or no more than 25 rule IDs; retain fail/pass fixtures and evidence-record comparison.

## Stop Rules

Stop on broad waivers, baselines, silent missing rules, or incompatible proof identity.
