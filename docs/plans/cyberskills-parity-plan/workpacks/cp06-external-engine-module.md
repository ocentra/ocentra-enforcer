# CP06 - Security Engine Consumer Contract

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP06 Security Engine Consumer Contract`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: boss-approved CyberSkills security-demand contract, consumer conformance fixtures, and `proof/cyberskills/cp06/**`; no generic harness surface
- deps: `CP00`, `UL07`
- tier: `P4 T1`

> Owner class: Sol/consumer architect; `tool-adapter-integrator` remains an external dependency owner.
> Batch limit: one security requirement slice and conformance fixture family; no real engine yet.
> Depends on: CP00 and Universal UL07 shared tool-routing acceptance.

## Owns

Universal UL07 and `tool-adapter-integrator` own the shared `enforcer-harness` process contract, typed tool registry/policy, generic fake/recorded adapters, normalized results, and generic gate integration. CP06 owns only CyberSkills requirement declarations and consumer conformance evidence. Its worker cannot edit the generic runner, registry, result schema, process policy, or workspace manifests.

## Where We Are

Recorded-output parsing and a generic severity gate exist, but there is no live runner, typed engine registry, process policy, resource control, or complete provenance.

## Where We Want To Be

Prove that the accepted UL07 contract can express CyberSkills security-engine requirements without weakening process policy or creating a Cyber-specific generic runner.

## Objective

Define the security consumer contract that CP07 and CP10 use: required tool capability, target kind, output protocol, policy needs, normalized evidence fields, and fail-closed availability semantics.

## Required interface and evidence

- Typed engine ID and output schema/version.
- Allowlists for executable identity, version constraint, arguments, target kinds, working directory, environment, network, and credential use.
- Deterministic timeout, process-tree termination, stdout/stderr/artifact limits, and redaction.
- Tool version, executable fingerprint where available, command/config digest, input/artifact hashes, timestamps, normalized findings, and coverage limitations.
- Distinct outcomes: ran, unavailable, policy-rejected, timed-out, errored, invalid-output.
- A conformance fixture proves the landed UL07 seam can carry each required field and failure outcome.

## Requirement Checklist

- [ ] No shell command string; executable and arguments are typed/separate.
- [ ] No inherited environment except an explicit allowlist.
- [ ] No network or credential access by default.
- [ ] Output parsing is bounded and schema-versioned.
- [ ] Unknown severity/taxonomy is explicit, not silently coerced.
- [ ] Engine file/location data is normalized without discarding provenance.
- [ ] Required-versus-optional availability is policy, not an adapter guess.
- [ ] A failed or absent engine cannot satisfy an engine component.
- [ ] Recorded fixtures cannot claim they represent a live run.
- [ ] Dogfood excludes only external artifacts/wrappers, not Rust gates.

## Acceptance And Proof

Consumer conformance tests cover every required outcome and prove that missing/version-mismatched/timed-out/failed/malformed tools cannot satisfy an engine-required component. The packet cites the accepted UL07 artifact and submits any generic contract gap back to `tool-adapter-integrator`; CP06 does not patch it.

## Stop conditions

Stop if the design requires arbitrary command execution, blanket environment inheritance, unbounded output, or one wrapper per skill.

## Parallel Ownership Notes

`tool-adapter-integrator` is the singleton writer for shared adapter seams and generic fixtures under UL07. CP06 owns only a disjoint security-demand/conformance packet. CP07 and CP10 consume the landed seam and own engine-specific paths; any generic gap blocks and returns to UL07.
