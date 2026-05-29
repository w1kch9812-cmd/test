# services/worker

Gongzzang-owned asynchronous jobs only.

Platform Core owns Catalog ETL, upstream public-data fetches, and source quota
management. This service must not fetch V-World/data.go.kr Catalog data or own
Catalog refresh schedules.

Allowed examples:

- listing moderation fan-out
- listing photo lifecycle cleanup
- notification delivery retry
- audit-log archival for Gongzzang-owned events
