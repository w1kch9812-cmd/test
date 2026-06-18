# MCP vs API

This document separates developer/agent exploration paths from production runtime paths.

## 1. Rule

MCP and agent tooling are allowed for development exploration only.

Production applications, services, crates, and packages must not depend on MCP servers, LLM SDKs, or agent-only connectors unless a future ADR creates a separate AI assistant boundary.

## 2. Production API Path

Production runtime uses explicit APIs and typed contracts:

```text
Browser
  -> Next.js routes/proxy
  -> Rust API
  -> Postgres / Redis / R2 / Platform Core published APIs
```

Important files:

- `services/api/src/app.rs`
- `apps/web/app/api/proxy/[...path]/route.ts`
- `docs/architecture/platform-integration/index.v1.json`
- `docs/architecture/platform-core-catalog-api-contract.v1.pin.json`

## 3. Agent Exploration Path

Agent sessions may inspect external systems and local code through MCP or browser automation.

Examples:

- repository audit
- source documentation lookup
- local browser inspection
- one-off research before creating an ADR or implementation plan

Agent-discovered facts must become code, ADR, registry, or explicit documentation before they influence production behavior.

## 4. Forbidden Production Coupling

Do not add MCP/LLM SDK dependencies to:

- `apps/web`
- `services/api`
- `services/outbox-publisher`
- `crates/domain`
- `crates/db`
- `packages/ui`

Do not make runtime correctness depend on an agent memory, chat history, local browser state, or MCP-only source.

## 5. Future AI Assistant Boundary

If Gongzzang adds AI features, they should enter through a separate approved boundary.

Expected shape:

```text
Gongzzang / Platform Core source records
  -> approved ingestion/indexing job
  -> vector/search/knowledge index
  -> AI assistant service
  -> product API
```

The AI service may use LLM SDKs, embeddings, vector search, and retrieval tooling. Main product domains still own canonical records and deletion/lifecycle rules.

## 6. Guardrails

When introducing AI or agent-facing code:

- write an ADR first;
- keep canonical business records in the owning service;
- keep generated summaries or embeddings as derived artifacts;
- add boundary checks before runtime dependency is introduced.

Relevant checks today:

```powershell
./scripts/ci/check-platform-core-boundary.ps1
./scripts/ci/check-platform-integration-policy.ps1
```
