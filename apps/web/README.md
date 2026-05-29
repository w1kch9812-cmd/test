# apps/web

Gongzzang web client.

## Scope

- Framework: Next.js App Router
- Role: browser UI and same-origin BFF proxy surface
- User-facing copy: typed i18n only
- LLM/MCP dependencies: forbidden in the runtime path

## Current Routes

- `/listings`: listing search/map surface
- `/api/proxy/*`: same-origin proxy to the Rust API
- `/api/auth/*`: authentication callback/session endpoints

## Boundaries

- Business rules stay in Rust domain crates and API services.
- Listing marker rendering uses the Gongzzang listing PBF source.
- PNU/parcel anchor ownership stays in `platform-core`.
