# Concurrent Session Role Split Handoff

Date: 2026-06-19
Repo: `C:\Users\admin\Desktop\gongzzang`

## Purpose

This handoff prevents two active agent sessions from editing the same ownership slice at the
same time.

The repo is currently clean at the shared boundary:

- `main == origin/main`
- worktree clean
- current HEAD: `a6a8b8f build: bind transition blockers to evidence`
- full guardrail suite was reported green at the commit boundary
- no Kafka or Kubernetes implementation was added
- no public-data collection was started

If any of those facts change, the active worker must report the new commit and changed files
before continuing.

## Current Work Split

### Session A: Bazel Transition Guardrail Worker

This worker may own:

- Bazel transition ratchet policy
- Bazel transition guardrail checker and tests
- ADR traceability for Bazel provisioning decisions
- transition exit-target evidence metadata

Reserved files and areas for that worker:

- `docs/architecture/verification-transition-ratchet.v1.json`
- `scripts/ci/check-bazel-transition-ratchet`
- `scripts/ci/transition-ratchet-bazel/**`
- `scripts/ci/check-bazel-transition-ratchet.tests`
- `scripts/ci/check-bazel-transition-ratchet-tests/**`
- Bazel guardrail target wiring in `BUILD.bazel` and `tools/bazel/**`
- `docs/superpowers/plans/2026-06-18-bazel-transition-ratchet.md`
- `docs/adr/0043-bazel-transition-provisioning-decisions.md`

### Session B: Product/Architecture Boundary Worker

This worker may own:

- repo-level architecture audit notes
- role split and handoff documentation
- Gongzzang vs Platform Core vs Dawneer ownership clarity
- non-implementation next-action ordering
- documentation consistency that does not edit Session A reserved files

Allowed low-conflict files and areas:

- `docs/superpowers/handoff/**`
- `docs/superpowers/next-actions.md`
- `docs/research/*gongzzang-current-project*`
- `docs/architecture/data-flow.md`
- `docs/architecture/layers.md`
- `docs/architecture/mcp-vs-api.md`
- `docs/architecture/geo-pipeline.md`
- `docs/architecture/caching.md`
- `docs/architecture/observability.md`

Before Session B edits code, migrations, generated artifacts, or any registry file, it must
re-check whether Session A is active and whether the target file is reserved.

## Shared Hard Stops

Neither session should do these without explicit user approval:

- start public-data collection
- create a DB migration
- delete or rewrite R2 objects
- run production AWS provisioning
- implement Kafka, Kubernetes, MSK, EKS, ECS, or Pulumi infrastructure changes
- move Platform Core-owned Catalog ingestion back into Gongzzang
- introduce direct V-World or data.go.kr Catalog API calls into Gongzzang runtime paths

## Current Technical Meaning

Gongzzang is not broken. The current state is high quality for boundary enforcement:

- Platform Core Catalog ownership is guarded.
- Gongzzang-owned listing marker ownership is documented and guarded.
- traffic/auth and lakehouse integration are registry-driven.
- generated and compatibility aggregate artifacts are guarded.
- Bazel transition state is explicit and cannot be claimed complete without evidence.

It is not the final enterprise form yet because:

- dependency SCA evidence still needs final Bazel-owned advisory evidence;
- coverage evidence still needs native Bazel evidence;
- migration verification still needs native Bazel/database service evidence;
- service e2e verification still needs native Bazel service orchestration evidence;
- production deployment and public-data collection have intentionally not run.

## Coordination Protocol

Before starting a new edit batch:

```bash
git status -sb --ahead-behind
git log --oneline --decorate -5
```

If the branch is behind `origin/main`, fetch and fast-forward before editing. If the worktree is
dirty, stop and identify whether the dirty files belong to the other session.

Before claiming a slice complete:

```bash
git diff --check
bash scripts/lefthook/file-line-limit.sh .
bash scripts/lefthook/check-forbidden-implementation-markers.sh .
bash scripts/ci/check-markdown-links.sh
```

For implementation changes, also run the smallest relevant targeted tests plus the relevant
guardrail suite. For Bazel transition changes, Session A must run the ratchet tests and
`//:guardrails_all`.

## Stop Hook Note

A stop hook failure after a clean push does not automatically mean the repo is dirty or broken.
Treat it as agent-runner metadata until `git status` or a project verification command proves a
repo problem.
