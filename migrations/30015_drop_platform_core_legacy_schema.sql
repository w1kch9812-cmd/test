-- V003_15: Drop Platform Core-owned legacy schema from the Gongzzang DB.
--
-- Approved by the user on 2026-05-28. This migration only targets the
-- Gongzzang database; it does not touch the Platform Core database.
-- Unexpected dependencies must fail instead of being silently removed.

drop table if exists api_health_check;
drop table if exists parcel_external_data;
drop table if exists pipeline_run;
drop table if exists pipeline_schedule;
