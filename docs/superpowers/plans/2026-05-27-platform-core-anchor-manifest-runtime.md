# Platform Core Anchor Manifest Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Gongzzang map runtime to the active Platform Core PNU anchor vector tile manifest.

**Architecture:** Platform Core vector tile manifest is the SSOT for public anchor tile URLs. Gongzzang registers Platform Core aggregate/exact anchor layers from that manifest and keeps Gongzzang listing marker tiles on the same-origin listing route.

**Tech Stack:** Next.js, TypeScript, Vitest, zod, Mapbox GL runtime inside Naver map.

---

## Task 1: Manifest Consumer

**Files:**
- Modify: `apps/web/lib/map/vector-tile-manifest.ts`
- Test: `apps/web/tests/unit/map/vector-tile-manifest.test.ts`

- [x] Write failing tests for `{object_key_prefix}` templates, anchor-only manifests, and root-relative template URL resolution.
- [x] Run `pnpm --filter @gongzzang/web exec vitest run tests/unit/map/vector-tile-manifest.test.ts`.
- [x] Update the manifest schema and URL materializer.
- [x] Re-run the same Vitest file until it passes.

## Task 2: Platform Core Anchor Style Registration

**Files:**
- Modify: `apps/web/lib/map/marker-tile-style.ts`
- Modify: `apps/web/lib/map/marker-tile-contract.ts`
- Test: `apps/web/tests/unit/map/marker-tile-style.test.ts`
- Test: `apps/web/tests/unit/map/marker-tile-contract.test.ts`

- [x] Write failing tests for aggregate/exact Platform Core anchor layer registration and same-origin listing marker tiles.
- [x] Run the two map marker Vitest files and confirm the new tests fail.
- [x] Replace the old Platform Core marker-contract dependency with manifest-backed registration.
- [x] Keep listing marker tile source construction in Gongzzang ownership.
- [x] Re-run the two Vitest files until they pass.

## Task 3: Runtime Wiring

**Files:**
- Modify: `apps/web/lib/map/listing-map-runtime.ts`
- Optional docs: `docs/frontend/listings-search.md`

- [x] Fetch the Platform Core vector tile manifest once during map setup.
- [x] Pass the manifest into polygon and anchor layer setup.
- [x] Treat polygon artifacts as optional until Platform Core publishes them.
- [x] Register click handling only on the exact `parcel_anchor` layer.
- [x] Update stale frontend troubleshooting text that references the retired marker contract.

## Task 4: Verification

**Files:**
- Existing changed files only.

- [x] Run focused Vitest files for map manifest/style/listing marker behavior.
- [x] Run `pnpm --filter @gongzzang/web typecheck`.
- [x] Run `pnpm --filter @gongzzang/web lint` if typecheck passes.
- [x] Report any remaining blocker clearly instead of claiming completion.
