# ADR-0043: Bazel Transition Provisioning Decisions

> ⛔ **[ADR-0044](./0044-bazel-transition-reconciliation.md)로 폐기됨 (2026-06-21 역전).** Bazel 전환은 취소됐고 **cargo(Rust) + pnpm/Turbo(프론트)가 영구 빌드 SSOT**다. 이 문서는 (취소된) Bazel 전환 provisioning 결정의 *역사적 기록*일 뿐 — 구현하지 말 것.

| | |
|---|---|
| Date | 2026-06-19 |
| Status | Superseded by ADR-0044 |
| Deciders | Gongzzang build platform |

## Context

ADR 0040 made Bazel the verification control plane, and ADR 0042 defines the
cross-repo Bazel-native build graph direction. Some Gongzzang CI checks still
run through explicit Bazel transition targets because they depend on external
advisory reads, CLI tools, Postgres/PostGIS service provisioning, or multi-service
orchestration.

The transition ratchet in
`docs/architecture/verification-transition-ratchet.v1.json` already records
owners, exit targets, approval gates, and planned evidence blockers. The missing
piece was that each approval gate's `decision_reference` could still be free text,
which weakens traceability and lets the transition state become a convention
instead of an auditable decision.

## Decision

Every Bazel transition approval gate must reference a tracked document under
`docs/`. For the current active transition set, this ADR is the decision reference
for:

- `external_advisory_collection`
- `browser_runtime_provisioning`
- `toolchain_provisioning`
- `database_service_provisioning`
- `service_orchestration_provisioning`

The transition ratchet checker must reject approval gates whose
`decision_reference` is not a `docs/...` file or whose referenced file does not
exist in the repository.

## Decisions By Gate

### External Advisory Collection

External advisory collection remains explicit. The SCA transition targets may run
only while they are declared as transition targets, and retiring them requires
pinned advisory evidence owned by Bazel. Until that evidence exists, the approval
gate stays unresolved and blocks `//:dependency_sca_evidence`.

### Toolchain Provisioning

Rust and Node language toolchains must stay Bazel-declared. Extra CLI tools such
as `cargo-deny`, `cargo-tarpaulin`, and `sqlx` may be used by transition runners
only while the transition ratchet records them as required commands. Retirement
requires the tool dependency to be represented as Bazel-owned evidence or as a
declared Bazel toolchain/provisioning target.

### Database Service Provisioning

DB-backed verification may rely on CI-provisioned Postgres/PostGIS only while the
transition ratchet records the `postgres` service and the workflow provisions it
in the same job. Retiring the migration transitions requires DB migration
execution to be represented by Bazel-owned service-backed verification, not an
untracked shell path.

### Service Orchestration Provisioning

Service e2e verification may rely on the current transition runner only while the
ratchet records its commands, services, readiness checks, and CI provisioning.
Retirement requires multi-service orchestration, readiness, log capture, and
failure surfacing to be represented as Bazel-owned execution.

### Browser Runtime Provisioning

Browser runtime provisioning is not active in the current transition target set,
but the gate remains registered for future browser-backed transitions. Any future
browser transition must keep the runtime provisioning decision traceable through
this ADR or a superseding ADR.

## Consequences

- Positive: transition blockers are now tied to tracked decision evidence.
- Positive: a free-text approval gate cannot silently become policy.
- Positive: future transition retirement work has a concrete audit trail.
- Cost: transition retirement still requires separate native Bazel evidence work.
- Cost: this ADR must be superseded if the provisioning model changes.

## Review Triggers

- A transition exit target changes from `planned` to `available`.
- A new transition approval gate is added.
- A CLI tool moves from CI install step to Bazel-managed toolchain or repository rule.
- DB or service orchestration moves from workflow service containers to Bazel-owned execution.
- External advisory evidence is pinned and SCA transitions are ready to retire.

## References

- [ADR 0040: Bazel-first build and verification control plane](./0040-bazel-first-build-verification-control-plane.md)
- [ADR 0042: Cross-Repo Bazel-Native Build Graph](./0042-cross-repo-bazel-native-build-graph.md)
- [Verification transition ratchet](../architecture/verification-transition-ratchet.v1.json)
