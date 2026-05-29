# Data Sources

This folder records source-boundary rules, not local Catalog client
implementations.

Catalog source integrations such as V-World and data.go.kr are owned by
Platform Core after M3.2 physical extraction. Gongzzang must consume Catalog
data through Platform Core contracts instead of direct source calls.

## Registered Sources

| Source | Owner | Gongzzang entry point | Document |
|---|---|---|---|
| V-World | Platform Core Catalog | Platform Core contracts only | [v-world.md](./v-world.md) |
| data.go.kr Catalog APIs | Platform Core Catalog | Platform Core contracts only | [data-go-kr.md](./data-go-kr.md) |
| Korean law API | Gongzzang only when product feature needs it | Direct API with breaker/audit/raw lineage | [korean-law.md](./korean-law.md) |
| NICE identity | Gongzzang auth/compliance | Direct provider integration | [nice-identity.md](./nice-identity.md) |
| Naver Maps | Gongzzang frontend/map UX | Approved maps integration | [naver-maps.md](./naver-maps.md) |

## Main-System Rule

Gongzzang-owned external calls must use timeout, retry, circuit breaker,
observability, and audit/logging rules. Catalog raw lineage for V-World and
data.go.kr belongs in Platform Core, not Gongzzang.

## Agent-Only Rule

MCP tools may be used for development exploration, but MCP/LLM SDKs must not be
imported by `apps/`, `services/`, `crates/`, or `packages/`.
