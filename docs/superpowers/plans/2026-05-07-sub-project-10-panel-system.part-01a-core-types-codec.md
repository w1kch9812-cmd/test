# SP10 Panel System - Part 01A: Core Types and Codec

Parent index: [SP10 Panel System - Part 01](./2026-05-07-sub-project-10-panel-system.part-01.md).
# SP10: Panel System — 지도 클릭 → 패널 stack 시스템 (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 지도 위 entity (필지/매물) 클릭 → typed-stack 패널 시스템을 SSS-grade로 구현. URL = SSOT, registry-driven 확장, xl breakpoint 단일 분기, 17 production rules 컴파일/CI 강제.

**Architecture:** Framework 본체 (`apps/web/lib/panel/*`) 는 kind-agnostic — typed stack codec, URL ↔ zustand 동기, breakpoint switch, 4-state shell. Kind 등록부 (`apps/web/components/panels/<kind>/register.ts`) 는 framework 외부. Backend 는 pure REST resource (`/api/parcels/:pnu`, `/api/buildings`) — "panel" 단어 모름. 통합은 `app/(authenticated)/listings/page.tsx` 의 `<ParcelInfoPanel>` 을 `<PanelRenderer>` 로 교체.

**Tech Stack:** Next.js 16 (App Router) · React 19 · TypeScript · zustand · TanStack Query · next-intl · Tailwind 4 · vitest · Playwright · axe-core · Axum (Rust) · sqlx · utoipa.

**Spec:** [docs/superpowers/specs/2026-05-07-sub-project-10-panel-system-design.md](../specs/2026-05-07-sub-project-10-panel-system-design.md) — 17 production rules + 3 FU + acceptance criteria 의 단일 출처.

**추정:** 5 영업일 (T1=1d, T2=0.5d, T3=1d, T4=1d, T5=0.5d, T6=1d).

---

## File Structure

### 신규 파일 (frontend)

```
apps/web/lib/panel/
├── types.ts                    PanelKind / PanelView / PanelStack / PanelStackEntry
├── codec.ts                    PanelStackCodec interface + g1 impl + Result
├── codec.test.ts               serialize/deserialize 회귀 (유효 + 깨진 URL)
├── registry.ts                 defineKind + registry singleton + lookup helpers
├── registry.test.ts            등록 / 중복 / 미등록 view 에러
├── use-panel-stack.ts          URL ↔ zustand 동기 (router.push 로만 mutate)
├── use-panel-stack.test.ts     hook 의 push/pop 이 router.push 호출 검증
├── panel-renderer.tsx          xl breakpoint switch (useMediaQuery)
├── side-by-side-stack.tsx      desktop renderer (top 2 + breadcrumb)
├── full-screen-stack.tsx       mobile renderer (top 1 + ‹back + depth)
├── panel-card.tsx              4-state shell, focus trap, ESC, error boundary
├── panel-card.test.tsx         4 state / ESC / focus restore
├── breadcrumb.tsx              sliding window 회색 항목 + mobile back
├── focus-trap.ts               focus push / restore hook
├── focus-trap.test.ts          push 시 focus 이동 / pop 시 복귀
└── telemetry.ts                Sentry breadcrumb / OTEL span / analytics

apps/web/components/panels/parcel/
├── summary.tsx                 Parcel summary card (PNU, 행정, 지목, 면적)
├── buildings.tsx               Parcel buildings list view
├── listings.tsx                Parcel listings list view
├── skeletons.tsx               Loading / Error / Empty 컴포넌트 모음
└── register.ts                 defineKind('parcel', {...})

apps/web/components/panels/listing/
├── summary.tsx                 Listing summary card (제목, 가격, 면적, photo)
├── skeletons.tsx               Loading / Error / Empty
└── register.ts                 defineKind('listing', {...})

apps/web/lib/api/
├── parcels.ts                  GET /api/parcels/:pnu client + zod schema
└── buildings.ts                GET /api/buildings?parcel_pnu=:pnu client + zod schema

apps/web/tests/e2e/
└── panel-system.spec.ts        spec § 10.2 e2e 시나리오

apps/web/tests/unit/
└── panel-extensibility.test.ts spec § 10.3 SSS 확장성 회귀
```

### 수정 파일 (frontend)

| 파일 | 변경 |
|---|---|
| `apps/web/app/(authenticated)/listings/page.tsx` | `<ParcelInfoPanel/>` 제거, `<PanelRenderer/>` 추가, kind register import |
| `apps/web/app/(authenticated)/listings/[id]/page.tsx` | 본문 → `redirect('/listings?p=listing:${id}.summary')` |
| `apps/web/components/listings/listing-map.tsx` | polygon click → `pushPanel(parcel.summary)`, marker click → `pushPanel(listing.summary)` |
| `apps/web/components/listings/listing-card.tsx` | `<Link href>` → onClick `pushPanel`, 가운데클릭 새 탭은 그대로 (서버 redirect 가 받음) |
| `apps/web/components/listings/listing-card-list.tsx` | filter pnu derive 를 `usePanelStack` 으로 |
| `apps/web/stores/listings.ts` | `filters.pnu` / `selectedListingId` 제거 |
| `apps/web/lib/listings/filters.ts` | `pnu` 필드 제거, `parseFiltersFromSearchParams` / `toSearchParams` 동조 |
| `apps/web/lib/i18n/ko.json` | `panels.parcel.*`, `panels.listing.*` namespace 추가 |
| `apps/web/biome.json` | (필요 시) `noRestrictedImports` 정책 추가 — `lib/panel/**` → `components/panels/**` 차단 |
| `lefthook.yml` | panel 커스텀 grep rule 3개 추가 (§10.1) |

### 삭제 파일

| 파일 | 사유 |
|---|---|
| `apps/web/components/listings/parcel-info-panel.tsx` | `<ParcelSummaryCard>` (registry) 가 대체 |
| `apps/web/tests/unit/listings/filters.test.ts` (부분) | `pnu` 검증 케이스 제거 (필드 자체 삭제) |

### 신규/수정 파일 (backend)

```
services/api/src/routes/
├── parcels.rs                  GET /api/parcels/:pnu (parcel-lookup 호출)
├── parcels_test.rs             integration test (NoOp + V-World wiremock)
├── buildings.rs                GET /api/buildings?parcel_pnu=:pnu
└── buildings_test.rs           integration test
```

수정: `services/api/src/main.rs` — 두 라우트 추가 + state 조립.

---

## Task 1: `lib/panel/` Framework Core

**목표:** Kind-agnostic framework 본체 — types, codec, registry, hook, panel-card, focus-trap, telemetry. UI renderer 는 T2 분리.

**Files:**
- Create: `apps/web/lib/panel/types.ts`
- Create: `apps/web/lib/panel/codec.ts`
- Create: `apps/web/lib/panel/codec.test.ts`
- Create: `apps/web/lib/panel/registry.ts`
- Create: `apps/web/lib/panel/registry.test.ts`
- Create: `apps/web/lib/panel/use-panel-stack.ts`
- Create: `apps/web/lib/panel/use-panel-stack.test.ts`
- Create: `apps/web/lib/panel/focus-trap.ts`
- Create: `apps/web/lib/panel/focus-trap.test.ts`
- Create: `apps/web/lib/panel/panel-card.tsx`
- Create: `apps/web/lib/panel/panel-card.test.tsx`
- Create: `apps/web/lib/panel/telemetry.ts`

### Step 1.1: Types

- [ ] **Step 1.1.1: Create `types.ts`**

```ts
// apps/web/lib/panel/types.ts

/**
 * SP10 Panel System — typed stack 추상화.
 * Spec § 3 Pattern E. 새 kind 추가 = `PanelKind` union 확장 + `PanelView<K>` 분기.
 * Framework 본체 (`lib/panel/*`) 는 kind 폴더 (`components/panels/*`) 를 import 하지 않음.
 */

export type PanelKind = 'parcel' | 'listing';

export type PanelView<K extends PanelKind> =
  | (K extends 'parcel' ? 'summary' | 'buildings' | 'listings' : never)
  | (K extends 'listing' ? 'summary' : never);

export type PanelStackEntry = {
  [K in PanelKind]: { kind: K; id: string; view: PanelView<K> };
}[PanelKind];

export interface PanelStack {
  v: 1;
  entries: PanelStackEntry[];
}

export const EMPTY_STACK: PanelStack = { v: 1, entries: [] };

/** depth 8 hard limit (spec § 14). */
export const PANEL_DEPTH_MAX = 8;
export const PANEL_DEPTH_WARN = 6;
```

- [ ] **Step 1.1.2: Commit**

```bash
git add apps/web/lib/panel/types.ts
git commit -m "feat(sp10-t1): panel/types — PanelKind/View/Stack discriminated union"
```

### Step 1.2: Codec (TDD)

- [ ] **Step 1.2.1: Write failing tests `codec.test.ts`**

```ts
// apps/web/lib/panel/codec.test.ts
import { describe, expect, it } from 'vitest';
import { g1Codec, ParseError } from './codec';
import type { PanelStack } from './types';

describe('g1Codec', () => {
  it('serializes single parcel.summary entry', () => {
    const stack: PanelStack = {
      v: 1,
      entries: [{ kind: 'parcel', id: '1168010100107370000', view: 'summary' }],
    };
    expect(g1Codec.serialize(stack)).toBe('parcel:1168010100107370000.summary');
  });

  it('serializes 2-entry chain with > separator', () => {
    const stack: PanelStack = {
      v: 1,
      entries: [
        { kind: 'parcel', id: '1168010100107370000', view: 'summary' },
        { kind: 'listing', id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee', view: 'summary' },
      ],
    };
    expect(g1Codec.serialize(stack)).toBe(
      'parcel:1168010100107370000.summary>listing:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.summary',
    );
  });

  it('serializes empty stack to empty string', () => {
    expect(g1Codec.serialize({ v: 1, entries: [] })).toBe('');
  });

  it('round-trips a 2-entry stack', () => {
    const s = 'parcel:1168010100107370000.summary>listing:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.summary';
    const parsed = g1Codec.deserialize(s);
    expect(parsed.ok).toBe(true);
    if (parsed.ok) expect(g1Codec.serialize(parsed.value)).toBe(s);
  });

  it('rejects unknown kind', () => {
    const r = g1Codec.deserialize('alien:abc.summary');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.UnknownKind);
  });

  it('rejects unknown view for parcel', () => {
    const r = g1Codec.deserialize('parcel:1168010100107370000.alienView');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.UnknownView);
  });

  it('rejects PNU pattern violation', () => {
    const r = g1Codec.deserialize('parcel:notapnu.summary');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.IdPatternViolation);
  });

  it('rejects malformed entry (missing dot)', () => {
    const r = g1Codec.deserialize('parcel:1168010100107370000');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.Malformed);
  });

  it('rejects depth > PANEL_DEPTH_MAX', () => {
    const long = Array.from({ length: 9 }, () => 'parcel:1168010100107370000.summary').join('>');
    const r = g1Codec.deserialize(long);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.DepthExceeded);
  });

  it('returns Malformed for empty input round-trip', () => {
    // empty string is a valid empty stack — caller decides which
    const r = g1Codec.deserialize('');
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value.entries).toHaveLength(0);
  });
});
```

- [ ] **Step 1.2.2: Run test → expect FAIL (file missing)**

Run: `cd apps/web && pnpm test lib/panel/codec`
Expected: `Cannot find module './codec'`

- [ ] **Step 1.2.3: Implement `codec.ts`**

```ts
// apps/web/lib/panel/codec.ts
import type { PanelKind, PanelStack, PanelStackEntry, PanelView } from './types';
import { PANEL_DEPTH_MAX } from './types';

/**
 * Spec § 5 — URL = SSOT. 모든 string 파싱은 본 파일만.
 * `string.split('>')` ad-hoc 파싱은 lefthook lint 가 차단 (T6).
 */

export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

export const ParseError = {
  Malformed: 'malformed',
  UnknownKind: 'unknown_kind',
  UnknownView: 'unknown_view',
  IdPatternViolation: 'id_pattern_violation',
  DepthExceeded: 'depth_exceeded',
} as const;
export type ParseError = (typeof ParseError)[keyof typeof ParseError];

interface KindMeta {
  views: ReadonlySet<string>;
  idPattern: RegExp;
}

/** SSOT for kind regex + valid views. spec § 5.3 + § 6. */
const KINDS: Record<PanelKind, KindMeta> = {
  parcel: {
    views: new Set(['summary', 'buildings', 'listings']),
    idPattern: /^\d{19}$/,
  },
  listing: {
    views: new Set(['summary']),
    idPattern: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
  },
};

const VALID_KINDS = Object.keys(KINDS) as PanelKind[];

function isPanelKind(s: string): s is PanelKind {
  return (VALID_KINDS as string[]).includes(s);
}

export interface PanelStackCodec {
  CURRENT_VERSION: 1;
  serialize(stack: PanelStack): string;
  deserialize(s: string): Result<PanelStack, ParseError>;
}

function serializeEntry(e: PanelStackEntry): string {
  return `${e.kind}:${e.id}.${e.view}`;
}

function deserializeEntry(raw: string): Result<PanelStackEntry, ParseError> {
  // grammar: kind ':' id '.' view
  const colon = raw.indexOf(':');
  if (colon < 1) return { ok: false, error: ParseError.Malformed };
  const kind = raw.slice(0, colon);
  const rest = raw.slice(colon + 1);
  const lastDot = rest.lastIndexOf('.');
  if (lastDot < 1) return { ok: false, error: ParseError.Malformed };
  const id = rest.slice(0, lastDot);
  const view = rest.slice(lastDot + 1);
  if (!id || !view) return { ok: false, error: ParseError.Malformed };
  if (!isPanelKind(kind)) return { ok: false, error: ParseError.UnknownKind };
  const meta = KINDS[kind];
  if (!meta.views.has(view)) return { ok: false, error: ParseError.UnknownView };
  if (!meta.idPattern.test(id)) return { ok: false, error: ParseError.IdPatternViolation };
  // Type-safe assembly: discriminated union narrows view per kind.
  return { ok: true, value: { kind, id, view: view as PanelView<PanelKind> } as PanelStackEntry };
}

export const g1Codec: PanelStackCodec = {
  CURRENT_VERSION: 1,
  serialize(stack: PanelStack): string {
    return stack.entries.map(serializeEntry).join('>');
  },
  deserialize(s: string): Result<PanelStack, ParseError> {
    if (s === '') return { ok: true, value: { v: 1, entries: [] } };
    const parts = s.split('>');
    if (parts.length > PANEL_DEPTH_MAX) {
      return { ok: false, error: ParseError.DepthExceeded };
    }
    const entries: PanelStackEntry[] = [];
    for (const p of parts) {
      const r = deserializeEntry(p);
      if (!r.ok) return r;
      entries.push(r.value);
    }
    return { ok: true, value: { v: 1, entries } };
  },
};
```

- [ ] **Step 1.2.4: Run test → expect PASS**

Run: `cd apps/web && pnpm test lib/panel/codec`
Expected: all tests pass.

- [ ] **Step 1.2.5: Commit**

```bash
git add apps/web/lib/panel/codec.ts apps/web/lib/panel/codec.test.ts
git commit -m "feat(sp10-t1): panel/codec — g1 grammar (kind:id.view>...) + Result + ParseError"
```
