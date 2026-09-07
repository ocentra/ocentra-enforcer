# `enforcer-syntax` ownership

`enforcer-syntax` is the sole owner of the shared language registry, structural
language specifications, parser dispatch, grammar bindings, and vendored
grammar bytes used by the workspace.

Consumers such as `enforcer-memory` may depend on these syntax facts, but this
crate does not own persistence, retrieval, embeddings, coordination, rule
execution, or UI concerns. Grammar and parser behavior must remain explicit:
unsupported inputs are represented by the existing `TextOnly` route rather
than silently receiving structural semantics.
