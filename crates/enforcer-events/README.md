# ocentra-eventing

Reusable Rust eventing primitives for Ocentra Parent runtime code.

## Phase 1 Proof

The reusable event bus merge gate is `scripts/test/eventing-runtime-proof.mjs`.
When it passes, it writes
`output/eventing-plan-proof/reusable-eventing-runtime/proof-summary.json` and
proves the generic crate runtime, delivery-decision helper, metrics/testkit,
queue/retry/timeout, request/response, journal/replay, lifecycle, source-safety,
topology, registry, fixture-parity, and compatibility rows without running
network, portal, service, product runtime, external transport, external relay,
decision-engine, AI, enforcement, or platform-adapter consumer proofs.

That proof file is absent in this checkout, so treat the path above as the
expected artifact location rather than current proof.

The full event-plan merge readiness gate is
`scripts/test/eventing-full-plan-proof.mjs`. That aggregate plan proof runs this
generic crate proof plus consumer proofs that show parent/controller,
child-agent, network, service, UI, command-boundary, and enforcement journal
paths consume the eventing contracts without moving product behavior into this
crate. Do not treat the broader route as proved in this checkout unless the
expected eventing-plan proof roots actually exist.

## Owns

- Validated event identifiers, correlation ids, aggregate keys, idempotency
  keys, source ids, subscriber ids, target handlers, and recorded timestamps.
- `DomainEvent` contracts, typed `EventEnvelope<E>` live dispatch, and
  `StoredEventEnvelope` serialization boundaries.
- Explicit `EventBus` instances owned by the runtime that constructs them.
- Sequential, concurrent, and aggregate-ordered typed dispatch with
  target-handler filtering, duplicate subscriber rejection, stored-envelope
  journal snapshots, exact handler reports, panic-isolation dead letters, and
  nested publish through typed `EventContext<E>`.
- Observable detached publish, awaitable publish reports, scoped
  `SubscriptionHandle` unsubscribe/drop behavior, and `EventRegistrar`
  ownership/dispose lifecycle.
- Handler execution policy for timeout and retry attempts, handler trace fields
  for event id/type/correlation/handler/outcome, and a real-subscription
  `EventRecorder<E>` testkit helper.
- `EventBus::clear_for_test` lifecycle for deterministic tests: it reports and
  clears local subscriptions, in-memory journal snapshots, dead letters,
  aggregate gates, queue/idempotency state, and pending request completions.
- `EventBus::shutdown` lifecycle for owned runtime shutdown: it supports
  production drain, production queued dead-letter, and explicit test-only queued
  drop modes, cancels pending local requests, clears subscriptions and
  aggregate gates, and rejects later publish/subscribe calls.
- Local bounded no-subscriber queue policy with observable drain reports,
  overflow rejection/dead-letter behavior, queue TTL expiry before dispatch,
  in-flight duplicate rejection, optional completed idempotency registry, and
  typed dead-letter event conversion.
- Local request/response completion with `RequestEvent::Response` type binding,
  response validation through `EventResponseContract`, timeout reporting,
  late-response and double-completion reports, and durable result-event tests
  kept separate from local completion.
- Injectable `EventClock` support with a system clock for runtime paths and a
  manual clock for deterministic TTL, deadline, retry, handler-timeout, and
  request-timeout tests without long wall-clock sleeps.
- `EventContractRegistry` descriptors for implemented event contracts, duplicate
  event type rejection, and deterministic generated Markdown registry docs.
- Typed event-family enum/wrapper variants for lineage patterns where one
  family subscriber handles concrete variants without downcasts, loose strings,
  or JSON shape inspection.
- Generated `EventTopologyManifest` proof docs for registered event contracts,
  explicit publishers, subscribers, family variants, orphan states, and
  accepted one-sided states.
- Generic `EventDeliveryDecisionProof` support for local-first delivery routes,
  typed subscriber filtering, bounded queue/TTL/dead-letter/idempotency
  backpressure metadata, retention policy refs, and external transport/relay
  requirements without implementing consumer transport.
- Executable `EventCompatibilityMatrix` proof docs that map Ocentra
  Games/TypeScript eventing lineage semantics to compatible Rust surfaces,
  intentional deviations, and manual-required external transport delivery
  scope.
- Shared TypeScript/Rust branded scalar fixture parity for eventing identifiers:
  Effect Schema brands and Rust newtypes accept and reject the same canonical
  fixture values.
- Durable `EventJournal` support with async NDJSON append, optional stable
  SHA-256 hash-chain records, recovery/replay tamper verification, selected
  journaling by event type/namespace/allowlist, replay cursors and filters,
  explicit projection-only replay mode, and journal before/after dispatch
  policy hooks.
- Immutable handler-facing `EventContext<E>` accessors so handlers can inspect
  typed envelopes, payloads, and publishers without receiving mutable payload
  references or payload-carried completion/resource handles.
- No production `.lock().await` in the reusable crate: registry, queue, request,
  journal state, and aggregate ordering use short synchronous state locks plus
  explicit async semaphore gates where ordering must cross awaits.

## Must Not Own

- Parent-specific event payloads or product policy.
- Network-only bus, external queue, request broker, or platform transport
  machinery.
- Portal UI business behavior.
- Hidden global singleton state.

## Current Gap

This crate does not yet implement external transport delivery, external relay
delivery, Parent-specific event contracts, cross-process transport shutdown,
platform adapter rollback execution, production retention/delete/export
behavior, or whole-repo source scanning for topology discovery. Consumers can
compose delivery decision proof with their own queue/idempotency/drop-audit
proof, as the network runtime does for row10a, but must keep live transport,
relay, and other consumer runtime claims manual-required until the matching
eventing workpacks are implemented and validated on top of the reusable bus.
