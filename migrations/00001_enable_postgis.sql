-- Enable PostGIS extension (required by 10001_core_tables.sql for geometry type).
-- Runs first because tests drop+create the DB before applying migrations,
-- which removes the extension that the docker-entrypoint pre-installed.
create extension if not exists postgis;
