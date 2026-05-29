# apps/worker

Reserved for Gongzzang-owned application jobs.

Catalog ETL and public-data ingestion belong to Platform Core. Gongzzang workers
may consume Platform Core published APIs, events, or immutable artifacts, but
must not fetch canonical Catalog source data directly.

Allowed examples:

- listing notification fan-out
- listing photo cleanup
- product analytics rollups over Gongzzang-owned events
