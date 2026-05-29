# crates/data-clients

This directory is reserved for Gongzzang-owned, non-Catalog external API
anti-corruption adapters only.

Catalog source adapters are not owned by this repo after the Platform Core
extraction. Do not recreate local clients here for V-World parcel data,
catalog data.go.kr APIs, Catalog raw capture, or public/reference spatial data
readers. Gongzzang must consume those through Platform Core published contracts,
event receivers, or approved read-model artifacts.

Allowed future adapters require an ADR and boundary update before code is added.
Examples may include Gongzzang-owned identity, law, map, or embedding providers
when they are not Catalog ownership paths.

## Rules

- Every external call uses Circuit Breaker, retry, timeout, and audit logging.
- API keys come from approved secret loading or environment variables only.
- Response lineage belongs to the owning service contract; Gongzzang must not
  add Catalog raw persistence tables or local raw capture crates.
- Adapters return Gongzzang-owned DTOs or ports; external schemas must not leak
  into domain crates.

See [docs/data-sources](../../docs/data-sources/README.md) and
[docs/backend/circuit-breaker.md](../../docs/backend/circuit-breaker.md).
