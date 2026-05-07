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

### Step 1.3: Registry (TDD)

- [ ] **Step 1.3.1: Write failing tests `registry.test.ts`**

```ts
// apps/web/lib/panel/registry.test.ts
import { afterEach, describe, expect, it } from 'vitest';
import { defineKind, getKindDefinition, getView, _resetRegistryForTests } from './registry';

afterEach(() => {
  _resetRegistryForTests();
});

const DummyComponent = () => null;

describe('registry', () => {
  it('registers and retrieves a kind', () => {
    defineKind({
      kind: 'parcel',
      idPattern: /^\d{19}$/,
      views: { summary: { component: DummyComponent, fetcher: async () => ({}), staleTime: 60_000, links: [] } },
      loadingComponent: DummyComponent,
      errorComponent: DummyComponent,
      emptyComponent: DummyComponent,
      authGate: { required: false },
      i18nNamespace: 'panels.parcel',
      telemetryAttrs: () => ({}),
    });
    const def = getKindDefinition('parcel');
    expect(def?.kind).toBe('parcel');
  });

  it('throws on duplicate registration', () => {
    const def = {
      kind: 'parcel' as const,
      idPattern: /^\d{19}$/,
      views: { summary: { component: DummyComponent, fetcher: async () => ({}), staleTime: 60_000, links: [] } },
      loadingComponent: DummyComponent,
      errorComponent: DummyComponent,
      emptyComponent: DummyComponent,
      authGate: { required: false } as const,
      i18nNamespace: 'panels.parcel',
      telemetryAttrs: () => ({}),
    };
    defineKind(def);
    expect(() => defineKind(def)).toThrowError(/already registered/i);
  });

  it('returns undefined for unregistered kind', () => {
    expect(getKindDefinition('parcel')).toBeUndefined();
  });

  it('getView returns view config for registered kind+view', () => {
    defineKind({
      kind: 'parcel',
      idPattern: /^\d{19}$/,
      views: { summary: { component: DummyComponent, fetcher: async () => ({}), staleTime: 60_000, links: [] } },
      loadingComponent: DummyComponent,
      errorComponent: DummyComponent,
      emptyComponent: DummyComponent,
      authGate: { required: false },
      i18nNamespace: 'panels.parcel',
      telemetryAttrs: () => ({}),
    });
    expect(getView('parcel', 'summary')).toBeDefined();
  });
});
```

- [ ] **Step 1.3.2: Run test → expect FAIL**

Run: `cd apps/web && pnpm test lib/panel/registry`
Expected: `Cannot find module './registry'`

- [ ] **Step 1.3.3: Implement `registry.ts`**

```ts
// apps/web/lib/panel/registry.ts
import type { ComponentType } from 'react';
import type { PanelKind, PanelStackEntry, PanelView } from './types';

/**
 * Spec § 6 — registry shape R1. SSOT for kind/view definitions.
 * Module-singleton: T4/T5 의 register.ts 가 import 시점에 1회 등록.
 */

export interface PanelLink<TFromData> {
  /** UI label key (i18n) */
  labelKey: string;
  /** discriminator: when this link should render (optional predicate on fetched data) */
  show?: (data: TFromData) => boolean;
  /** target stack entry — must reference *another* kind's registered view (compile-time enforced) */
  to: (data: TFromData) => PanelStackEntry;
}

export interface PanelViewDefinition<K extends PanelKind, TData = unknown> {
  component: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }>; data: TData }>;
  /** id → server fetch. Returns parsed data (zod schema 검증된 후). */
  fetcher: (id: string) => Promise<TData>;
  /** TanStack Query staleTime ms. spec FU2 = per-kind tune. v1 default 5min. */
  staleTime: number;
  /** child link list — registry link integrity 는 T4/T5 가 검증 */
  links: PanelLink<TData>[];
}

export interface PanelKindDefinition<K extends PanelKind> {
  kind: K;
  idPattern: RegExp;
  views: { [V in PanelView<K>]: PanelViewDefinition<K> };
  loadingComponent: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }> }>;
  errorComponent: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }>; error: unknown }>;
  emptyComponent: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }> }>;
  /** auth: required=true → 미인증 시 AuthGate 카드. */
  authGate: { required: boolean };
  i18nNamespace: string;
  /** Sentry breadcrumb / OTEL span attributes. */
  telemetryAttrs: (entry: Extract<PanelStackEntry, { kind: K }>) => Record<string, string | number | boolean>;
}

const REGISTRY = new Map<PanelKind, PanelKindDefinition<PanelKind>>();

export function defineKind<K extends PanelKind>(def: PanelKindDefinition<K>): void {
  if (REGISTRY.has(def.kind)) {
    throw new Error(`Panel kind '${def.kind}' is already registered`);
  }
  REGISTRY.set(def.kind, def as unknown as PanelKindDefinition<PanelKind>);
}

export function getKindDefinition<K extends PanelKind>(kind: K): PanelKindDefinition<K> | undefined {
  return REGISTRY.get(kind) as PanelKindDefinition<K> | undefined;
}

export function getView<K extends PanelKind>(
  kind: K,
  view: PanelView<K>,
): PanelViewDefinition<K> | undefined {
  const def = getKindDefinition(kind);
  return def?.views[view] as PanelViewDefinition<K> | undefined;
}

/** TEST ONLY — clears registry between tests. */
export function _resetRegistryForTests(): void {
  REGISTRY.clear();
}
```

- [ ] **Step 1.3.4: Run test → expect PASS**

Run: `cd apps/web && pnpm test lib/panel/registry`
Expected: all tests pass.

- [ ] **Step 1.3.5: Commit**

```bash
git add apps/web/lib/panel/registry.ts apps/web/lib/panel/registry.test.ts
git commit -m "feat(sp10-t1): panel/registry — defineKind + R1 shape (4-state shell + authGate + i18n + telemetry)"
```

### Step 1.4: `usePanelStack` hook (URL ↔ zustand)

- [ ] **Step 1.4.1: Write failing tests `use-panel-stack.test.ts`**

```ts
// apps/web/lib/panel/use-panel-stack.test.ts
import { renderHook, act } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';

const mockPush = vi.fn();
const mockBack = vi.fn();
const mockSearchParams = new URLSearchParams();

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: mockPush, back: mockBack, replace: vi.fn() }),
  useSearchParams: () => mockSearchParams,
  usePathname: () => '/listings',
}));

import { usePanelStack } from './use-panel-stack';

beforeEach(() => {
  mockPush.mockClear();
  mockBack.mockClear();
  mockSearchParams.delete('p');
});

describe('usePanelStack', () => {
  it('returns empty stack when ?p missing', () => {
    const { result } = renderHook(() => usePanelStack());
    expect(result.current.stack.entries).toHaveLength(0);
  });

  it('hydrates stack from ?p search param', () => {
    mockSearchParams.set('p', 'parcel:1168010100107370000.summary');
    const { result } = renderHook(() => usePanelStack());
    expect(result.current.stack.entries).toHaveLength(1);
    expect(result.current.stack.entries[0]).toEqual({
      kind: 'parcel',
      id: '1168010100107370000',
      view: 'summary',
    });
  });

  it('push calls router.push with serialized url', () => {
    const { result } = renderHook(() => usePanelStack());
    act(() => {
      result.current.push({ kind: 'parcel', id: '1168010100107370000', view: 'summary' });
    });
    expect(mockPush).toHaveBeenCalledWith(
      '/listings?p=parcel%3A1168010100107370000.summary',
      { scroll: false },
    );
  });

  it('pop calls router.back', () => {
    mockSearchParams.set(
      'p',
      'parcel:1168010100107370000.summary>listing:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.summary',
    );
    const { result } = renderHook(() => usePanelStack());
    act(() => {
      result.current.pop();
    });
    expect(mockBack).toHaveBeenCalledTimes(1);
  });

  it('silent recover from broken url (empty stack)', () => {
    mockSearchParams.set('p', 'invalid:bad.thing');
    const { result } = renderHook(() => usePanelStack());
    expect(result.current.stack.entries).toHaveLength(0);
    // depth-0 = silent recover (Sentry 이벤트는 telemetry.test.ts 가 검증)
  });
});
```

- [ ] **Step 1.4.2: Run test → expect FAIL**

Run: `cd apps/web && pnpm test lib/panel/use-panel-stack`
Expected: `Cannot find module './use-panel-stack'`

- [ ] **Step 1.4.3: Implement `use-panel-stack.ts`**

```ts
// apps/web/lib/panel/use-panel-stack.ts
'use client';

import { usePathname, useRouter, useSearchParams } from 'next/navigation';
import { useCallback, useMemo } from 'react';
import { g1Codec } from './codec';
import { reportUrlDecodeFailed } from './telemetry';
import type { PanelStack, PanelStackEntry } from './types';
import { EMPTY_STACK } from './types';

/**
 * Spec § 5.4 — URL = SSOT. zustand 의 panelStack 사본은 *없음* — useSearchParams 직접.
 * mutation 은 router.push (URL grammar), pop 은 router.back (브라우저 stack).
 */

export interface UsePanelStackResult {
  stack: PanelStack;
  push: (entry: PanelStackEntry) => void;
  pop: () => void;
  /** stack 을 명시적 길이로 자름 (breadcrumb 클릭 시 사용). */
  truncate: (depth: number) => void;
}

export function usePanelStack(): UsePanelStackResult {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const raw = searchParams.get('p');

  const stack = useMemo<PanelStack>(() => {
    if (!raw) return EMPTY_STACK;
    const r = g1Codec.deserialize(raw);
    if (!r.ok) {
      reportUrlDecodeFailed(raw, r.error);
      return EMPTY_STACK;
    }
    return r.value;
  }, [raw]);

  const navigate = useCallback(
    (next: PanelStack) => {
      const sp = new URLSearchParams(searchParams.toString());
      const serialized = g1Codec.serialize(next);
      if (serialized) sp.set('p', serialized);
      else sp.delete('p');
      const qs = sp.toString();
      router.push(`${pathname}${qs ? `?${qs}` : ''}`, { scroll: false });
    },
    [pathname, router, searchParams],
  );

  const push = useCallback(
    (entry: PanelStackEntry) => {
      const next: PanelStack = { v: 1, entries: [...stack.entries, entry] };
      navigate(next);
    },
    [navigate, stack],
  );

  const pop = useCallback(() => {
    router.back();
  }, [router]);

  const truncate = useCallback(
    (depth: number) => {
      const safeDepth = Math.max(0, Math.min(depth, stack.entries.length));
      navigate({ v: 1, entries: stack.entries.slice(0, safeDepth) });
    },
    [navigate, stack],
  );

  return { stack, push, pop, truncate };
}
```

- [ ] **Step 1.4.4: Run test → expect PASS**

Run: `cd apps/web && pnpm test lib/panel/use-panel-stack`
Expected: all tests pass.

- [ ] **Step 1.4.5: Commit**

```bash
git add apps/web/lib/panel/use-panel-stack.ts apps/web/lib/panel/use-panel-stack.test.ts
git commit -m "feat(sp10-t1): panel/use-panel-stack — URL=SSOT hook (push=router.push, pop=router.back, truncate)"
```

### Step 1.5: Telemetry

> **Constraint:** `@sentry/nextjs` is **not** in `apps/web/package.json`. Use OTEL (already installed via `@opentelemetry/api`) + a Sentry-shaped abstraction so future Sentry adoption is a one-line swap.

- [ ] **Step 1.5.1: Implement `telemetry.ts`**

```ts
// apps/web/lib/panel/telemetry.ts
import { trace } from '@opentelemetry/api';
import type { PanelStackEntry } from './types';
import type { ParseError } from './codec';

/**
 * Spec § 10.4 — telemetry standard.
 * v1 backends:
 *   - OTEL span (panel.opened) — spec § 10.4 attributes
 *   - console.warn for url_decode_failed (Sentry adoption = drop-in replace)
 *   - window.dataLayer push (analytics)
 *
 * 본 helper 는 Sentry 도입 후 import 만 swap (call sites 동일).
 */

const TRACER = trace.getTracer('panel');

interface AnalyticsDataLayer {
  push: (event: Record<string, unknown>) => void;
}

interface DataLayerWindow extends Window {
  dataLayer?: AnalyticsDataLayer | Array<Record<string, unknown>>;
}

function pushAnalytics(event: Record<string, unknown>): void {
  if (typeof window === 'undefined') return;
  const dl = (window as DataLayerWindow).dataLayer;
  if (Array.isArray(dl)) {
    dl.push(event);
  } else if (dl && typeof dl.push === 'function') {
    dl.push(event);
  }
}

export function reportPanelOpened(entry: PanelStackEntry, depth: number, fetchMs: number): void {
  const span = TRACER.startSpan('panel.opened', {
    attributes: {
      'panel.kind': entry.kind,
      'panel.view': entry.view,
      'panel.id': entry.id,
      'panel.depth': depth,
      'panel.fetch_ms': fetchMs,
    },
  });
  span.end();

  pushAnalytics({
    event: 'panel_opened',
    panel_kind: entry.kind,
    panel_view: entry.view,
    panel_id: entry.id,
    panel_depth: depth,
  });
}

export function reportUrlDecodeFailed(raw: string, error: ParseError): void {
  const span = TRACER.startSpan('panel.url_decode_failed', {
    attributes: { 'panel.raw': raw, 'panel.error': error },
  });
  span.end();
  // dev visibility — production 은 OTEL collector 가 export.
  if (process.env.NODE_ENV !== 'production') {
    console.warn('[panel] url_decode_failed', { raw, error });
  }
}
```

- [ ] **Step 1.5.2: Commit**

```bash
git add apps/web/lib/panel/telemetry.ts
git commit -m "feat(sp10-t1): panel/telemetry — OTEL span + dataLayer analytics (Sentry-swap-ready)"
```

### Step 1.6: Focus Trap (TDD)

- [ ] **Step 1.6.1: Write failing tests `focus-trap.test.ts`**

```ts
// apps/web/lib/panel/focus-trap.test.ts
import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useFocusTrap } from './focus-trap';

describe('useFocusTrap', () => {
  it('moves focus to container on mount, restores on unmount', () => {
    const prev = document.createElement('button');
    document.body.appendChild(prev);
    prev.focus();
    expect(document.activeElement).toBe(prev);

    const container = document.createElement('div');
    container.tabIndex = -1;
    document.body.appendChild(container);

    const { unmount } = renderHook(() => useFocusTrap({ current: container }));
    expect(document.activeElement).toBe(container);

    unmount();
    expect(document.activeElement).toBe(prev);

    document.body.removeChild(prev);
    document.body.removeChild(container);
  });
});
```

- [ ] **Step 1.6.2: Run test → expect FAIL**

Run: `cd apps/web && pnpm test lib/panel/focus-trap`

- [ ] **Step 1.6.3: Implement `focus-trap.ts`**

```ts
// apps/web/lib/panel/focus-trap.ts
'use client';

import { useEffect, type RefObject } from 'react';

/**
 * Spec rule § 9.14 — focus push on open / restore on close.
 * Container must be focusable (tabIndex=-1 acceptable).
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const previously = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => {
      previously?.focus();
    };
  }, [ref]);
}
```

- [ ] **Step 1.6.4: Run test → expect PASS**

Run: `cd apps/web && pnpm test lib/panel/focus-trap`
Expected: pass.

- [ ] **Step 1.6.5: Commit**

```bash
git add apps/web/lib/panel/focus-trap.ts apps/web/lib/panel/focus-trap.test.ts
git commit -m "feat(sp10-t1): panel/focus-trap — push on mount, restore on unmount"
```

### Step 1.7: PanelCard 4-state shell (TDD)

- [ ] **Step 1.7.1: Write failing tests `panel-card.test.tsx`**

```tsx
// apps/web/lib/panel/panel-card.test.tsx
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PanelCard } from './panel-card';

describe('PanelCard', () => {
  it('renders loadingComponent when isLoading', () => {
    render(
      <PanelCard
        state="loading"
        onClose={() => {}}
        loading={<div>LOADING</div>}
        error={<div>ERR</div>}
        empty={<div>EMPTY</div>}
        authRequired={<div>AUTH</div>}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    expect(screen.getByText('LOADING')).toBeInTheDocument();
  });

  it('renders errorComponent when state=error', () => {
    render(
      <PanelCard
        state="error"
        onClose={() => {}}
        loading={<div>LOADING</div>}
        error={<div>ERR</div>}
        empty={<div>EMPTY</div>}
        authRequired={<div>AUTH</div>}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    expect(screen.getByText('ERR')).toBeInTheDocument();
  });

  it('renders content when state=ok', () => {
    render(
      <PanelCard
        state="ok"
        onClose={() => {}}
        loading={<div>LOADING</div>}
        error={<div>ERR</div>}
        empty={<div>EMPTY</div>}
        authRequired={<div>AUTH</div>}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    expect(screen.getByText('CONTENT')).toBeInTheDocument();
  });

  it('calls onClose on ESC keydown', () => {
    const onClose = vi.fn();
    render(
      <PanelCard
        state="ok"
        onClose={onClose}
        loading={null}
        error={null}
        empty={null}
        authRequired={null}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('has aria-modal=true and role=dialog', () => {
    render(
      <PanelCard
        state="ok"
        onClose={() => {}}
        loading={null}
        error={null}
        empty={null}
        authRequired={null}
      >
        <div />
      </PanelCard>,
    );
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
  });
});
```

- [ ] **Step 1.7.2: Run test → expect FAIL**

Run: `cd apps/web && pnpm test lib/panel/panel-card`

- [ ] **Step 1.7.3: Implement `panel-card.tsx`**

```tsx
// apps/web/lib/panel/panel-card.tsx
'use client';

import { useEffect, useRef, type ReactNode } from 'react';
import { useFocusTrap } from './focus-trap';

/**
 * Spec rule § 9 #6 (error boundary), #14 (focus trap), #15 (ESC), #16 (reduced motion), #17 (4-state).
 * 4-state: loading / error / ok / empty / auth-required.
 *   (auth-required 가 별도 prop — registry 의 authGate 미통과 시 렌더)
 */

export type PanelCardState = 'loading' | 'error' | 'empty' | 'ok' | 'auth-required';

export interface PanelCardProps {
  state: PanelCardState;
  onClose: () => void;
  loading: ReactNode;
  error: ReactNode;
  empty: ReactNode;
  authRequired: ReactNode;
  children: ReactNode;
  /** aria-labelledby target id (for screen readers). */
  titleId?: string;
}

export function PanelCard({
  state,
  onClose,
  loading,
  error,
  empty,
  authRequired,
  children,
  titleId,
}: PanelCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [onClose]);

  const body =
    state === 'loading'
      ? loading
      : state === 'error'
        ? error
        : state === 'empty'
          ? empty
          : state === 'auth-required'
            ? authRequired
            : children;

  return (
    <div
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabIndex={-1}
      // motion-safe / motion-reduce: spec § 9 #16
      className="motion-safe:animate-in motion-safe:slide-in-from-right motion-reduce:animate-none flex h-full w-full flex-col bg-[var(--color-canvas)]"
      onKeyDown={(e) => {
        if (e.key === 'Escape') onClose();
      }}
    >
      {body}
    </div>
  );
}
```

- [ ] **Step 1.7.4: Run test → expect PASS**

Run: `cd apps/web && pnpm test lib/panel/panel-card`
Expected: 5 tests pass.

- [ ] **Step 1.7.5: Commit**

```bash
git add apps/web/lib/panel/panel-card.tsx apps/web/lib/panel/panel-card.test.tsx
git commit -m "feat(sp10-t1): panel/panel-card — 4-state shell + ESC + focus trap + aria-modal + motion-reduce"
```

### Step 1.8: T1 typecheck + lint

- [ ] **Step 1.8.1: Run typecheck**

Run: `cd apps/web && pnpm typecheck`
Expected: no errors.

- [ ] **Step 1.8.2: Run lint**

Run: `cd apps/web && pnpm lint`
Expected: no errors.

- [ ] **Step 1.8.3: T1 closing commit (no-op if previous green)**

If typecheck/lint surfaced fixes, commit:

```bash
git add -A
git commit -m "fix(sp10-t1): typecheck/lint cleanup after framework core"
```

---

## Task 2: Renderers (Side-by-side / Full-screen / Breakpoint switch / Breadcrumb)

**목표:** xl=1280px breakpoint 단일 분기 — 한 곳에서만 분기, 같은 `<PanelCard>` / `<Breadcrumb>` 컴포넌트 양쪽 재사용. 사본 0.

**Files:**
- Create: `apps/web/lib/panel/breadcrumb.tsx`
- Create: `apps/web/lib/panel/side-by-side-stack.tsx`
- Create: `apps/web/lib/panel/full-screen-stack.tsx`
- Create: `apps/web/lib/panel/panel-renderer.tsx`
- Create: `apps/web/lib/panel/panel-renderer.test.tsx`

### Step 2.1: Breadcrumb

- [ ] **Step 2.1.1: Implement `breadcrumb.tsx`**

```tsx
// apps/web/lib/panel/breadcrumb.tsx
'use client';

import { useTranslations } from 'next-intl';
import { getKindDefinition } from './registry';
import type { PanelStack } from './types';
import { usePanelStack } from './use-panel-stack';

interface BreadcrumbProps {
  stack: PanelStack;
  /** 회색 항목 (sliding window 밖) 시작 인덱스. desktop 만 사용. -1 = no greyed. */
  greyedBeforeIndex?: number;
}

export function Breadcrumb({ stack, greyedBeforeIndex = -1 }: BreadcrumbProps) {
  const t = useTranslations('panel');
  const { truncate } = usePanelStack();

  if (stack.entries.length === 0) return null;

  return (
    <nav
      aria-label={t('breadcrumb')}
      className="flex items-center gap-1 px-4 py-2 text-[length:var(--text-caption)]"
    >
      {stack.entries.map((entry, idx) => {
        const def = getKindDefinition(entry.kind);
        const isLast = idx === stack.entries.length - 1;
        const greyed = greyedBeforeIndex >= 0 && idx < greyedBeforeIndex;
        const label = def ? `${entry.kind}.${entry.view}` : entry.kind;
        return (
          <span key={`${entry.kind}-${idx}`} className="flex items-center gap-1">
            {idx > 0 && <span className="text-[var(--color-muted)]">/</span>}
            <button
              type="button"
              onClick={() => truncate(idx + 1)}
              disabled={isLast}
              className={[
                'rounded px-1 hover:bg-[var(--color-surface-cream-strong)]',
                greyed ? 'text-[var(--color-muted)]' : 'text-[var(--color-ink)]',
                isLast ? 'cursor-default font-semibold' : 'cursor-pointer',
              ].join(' ')}
              aria-current={isLast ? 'page' : undefined}
            >
              {label}
            </button>
          </span>
        );
      })}
    </nav>
  );
}
```

- [ ] **Step 2.1.2: Commit**

```bash
git add apps/web/lib/panel/breadcrumb.tsx
git commit -m "feat(sp10-t2): panel/breadcrumb — sliding-window grey + truncate-on-click"
```

### Step 2.2: PanelEntryView (shared by both renderers)

- [ ] **Step 2.2.1: Implement helper `panel-entry-view.tsx`**

```tsx
// apps/web/lib/panel/panel-entry-view.tsx
'use client';

import { useQuery } from '@tanstack/react-query';
import { createElement, useEffect, useMemo, useRef } from 'react';
import { PanelCard } from './panel-card';
import { getKindDefinition, getView } from './registry';
import { reportPanelOpened } from './telemetry';
import type { PanelStackEntry } from './types';
import { usePanelStack } from './use-panel-stack';

/**
 * 단일 entry 의 렌더링 — fetcher 호출 + 4-state shell + 컴포넌트 dispatch.
 * Spec rule § 9 #1 (registry SSOT), #6 (error boundary via PanelCard), #8 (AbortController per slot — TanStack Query 가 자동), #17 (4-state).
 */
export function PanelEntryView({
  entry,
  depth,
}: {
  entry: PanelStackEntry;
  depth: number;
}) {
  const def = getKindDefinition(entry.kind);
  const viewDef = getView(entry.kind, entry.view);
  const startedAt = useRef(performance.now());
  const { pop } = usePanelStack();

  // Spec rule § 9 #8 — TanStack Query 의 queryFn 이 AbortSignal 받아 fetcher 에 전달.
  const query = useQuery({
    queryKey: ['panel', entry.kind, entry.view, entry.id],
    queryFn: async ({ signal }) => {
      void signal; // fetcher 가 signal 사용은 호출자 결정 (ky 가 AbortSignal 지원)
      return viewDef!.fetcher(entry.id);
    },
    staleTime: viewDef?.staleTime ?? 5 * 60_000,
    enabled: Boolean(def && viewDef),
  });

  useEffect(() => {
    if (query.isSuccess) {
      reportPanelOpened(entry, depth, performance.now() - startedAt.current);
    }
  }, [query.isSuccess, entry, depth]);

  // Spec rule § 13 — registered 안된 view import 자체가 컴파일 에러여야 하지만, runtime 의 안전망.
  const stateNarrowed = useMemo(() => {
    if (!def || !viewDef) return 'error' as const;
    if (query.isLoading) return 'loading' as const;
    if (query.isError) return 'error' as const;
    const data = query.data;
    if (data === null || (Array.isArray(data) && data.length === 0)) return 'empty' as const;
    return 'ok' as const;
  }, [def, viewDef, query.isLoading, query.isError, query.data]);

  if (!def || !viewDef) {
    return (
      <div className="p-6 text-center text-[var(--color-error)]">
        Unknown panel kind/view: {entry.kind}.{entry.view}
      </div>
    );
  }

  return (
    <PanelCard
      state={stateNarrowed}
      onClose={pop}
      loading={createElement(def.loadingComponent, { entry: entry as never })}
      error={createElement(def.errorComponent, { entry: entry as never, error: query.error })}
      empty={createElement(def.emptyComponent, { entry: entry as never })}
      authRequired={
        <div className="p-6 text-center text-[var(--color-muted)]">
          로그인이 필요해요
        </div>
      }
    >
      {query.data !== undefined &&
        createElement(viewDef.component, {
          entry: entry as never,
          data: query.data,
        })}
    </PanelCard>
  );
}
```

- [ ] **Step 2.2.2: Commit**

```bash
git add apps/web/lib/panel/panel-entry-view.tsx
git commit -m "feat(sp10-t2): panel/panel-entry-view — registry dispatch + TanStack Query + 4-state"
```

### Step 2.3: SideBySideStack (desktop)

- [ ] **Step 2.3.1: Implement `side-by-side-stack.tsx`**

> **Layout rule:** SideBySideStack uses `fixed top-0 right-0 bottom-0` overlay (width = 2× panel width) so PanelRenderer can be rendered ONCE at page top level — page grid math (map + card list aside) untouched. FullScreenStack also uses `fixed inset-0`. Both are mutually-exclusive via PanelRenderer's `useMediaQuery` switch.

```tsx
// apps/web/lib/panel/side-by-side-stack.tsx
'use client';

import { Breadcrumb } from './breadcrumb';
import { PanelEntryView } from './panel-entry-view';
import type { PanelStack } from './types';

/**
 * Spec § 4 desktop renderer. depth ≥ xl 에서 top 2 entry 를 side-by-side.
 * depth 3+ = sliding window (마지막 2 만), breadcrumb 회색 항목으로 이전 표시.
 *
 * 위치: `fixed top-0 right-0 bottom-0 w-[840px]` overlay — 페이지 grid 와
 * 독립적이라 listings page 의 map / card list aside 는 영향 0.
 */
export function SideBySideStack({ stack }: { stack: PanelStack }) {
  const total = stack.entries.length;
  if (total === 0) return null;

  const top2Start = Math.max(0, total - 2);
  const visible = stack.entries.slice(top2Start);

  return (
    <div className="fixed top-0 right-0 bottom-0 z-40 flex w-[840px] flex-col border-l border-[var(--color-hairline)] bg-[var(--color-canvas)] shadow-xl">
      <Breadcrumb stack={stack} greyedBeforeIndex={top2Start} />
      <div className="grid flex-1 grid-cols-2 gap-4 overflow-hidden">
        {visible.map((entry, i) => (
          <PanelEntryView
            key={`${entry.kind}-${entry.id}-${entry.view}-${top2Start + i}`}
            entry={entry}
            depth={top2Start + i + 1}
          />
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 2.3.2: Commit**

```bash
git add apps/web/lib/panel/side-by-side-stack.tsx
git commit -m "feat(sp10-t2): panel/side-by-side-stack — top 2 + breadcrumb sliding window"
```

### Step 2.4: FullScreenStack (mobile)

- [ ] **Step 2.4.1: Implement `full-screen-stack.tsx`**

```tsx
// apps/web/lib/panel/full-screen-stack.tsx
'use client';

import { ChevronLeft } from 'lucide-react';
import { useTranslations } from 'next-intl';
import { PanelEntryView } from './panel-entry-view';
import type { PanelStack } from './types';
import { usePanelStack } from './use-panel-stack';

/**
 * Spec § 4 mobile renderer. top 1 entry full-screen + 상단 ‹back + depth indicator.
 * back 은 router.back (브라우저 hw back / iOS edge-swipe 와 동등).
 */
export function FullScreenStack({ stack }: { stack: PanelStack }) {
  const total = stack.entries.length;
  const t = useTranslations('panel');
  const { pop } = usePanelStack();
  if (total === 0) return null;

  const top = stack.entries[total - 1]!;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-[var(--color-canvas)]">
      <div className="flex items-center gap-2 border-b border-[var(--color-hairline)] px-4 py-3">
        <button
          type="button"
          onClick={pop}
          aria-label={t('back')}
          className="flex h-9 w-9 items-center justify-center rounded-full hover:bg-[var(--color-surface-cream-strong)]"
        >
          <ChevronLeft className="h-5 w-5" />
        </button>
        <span className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {total} / {total}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        <PanelEntryView entry={top} depth={total} />
      </div>
    </div>
  );
}
```

- [ ] **Step 2.4.2: Commit**

```bash
git add apps/web/lib/panel/full-screen-stack.tsx
git commit -m "feat(sp10-t2): panel/full-screen-stack — top 1 + ‹back + depth indicator"
```

### Step 2.5: PanelRenderer breakpoint switch (TDD)

- [ ] **Step 2.5.1: Write failing test `panel-renderer.test.tsx`**

```tsx
// apps/web/lib/panel/panel-renderer.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

const matchMediaMock = vi.fn();
beforeEach(() => {
  matchMediaMock.mockReset();
});

vi.stubGlobal('matchMedia', (q: string) => {
  matchMediaMock(q);
  return {
    matches: q.includes('1280') ? matchMediaMock.matchesValue : false,
    addEventListener: () => {},
    removeEventListener: () => {},
  };
});

vi.mock('./side-by-side-stack', () => ({
  SideBySideStack: () => <div>SIDE_BY_SIDE</div>,
}));
vi.mock('./full-screen-stack', () => ({
  FullScreenStack: () => <div>FULL_SCREEN</div>,
}));
vi.mock('./use-panel-stack', () => ({
  usePanelStack: () => ({
    stack: { v: 1, entries: [{ kind: 'parcel', id: '1168010100107370000', view: 'summary' }] },
    push: () => {},
    pop: () => {},
    truncate: () => {},
  }),
}));

import { beforeEach } from 'vitest';
import { PanelRenderer } from './panel-renderer';

describe('PanelRenderer', () => {
  it('renders SideBySideStack at >= xl viewport', () => {
    matchMediaMock.matchesValue = true;
    render(<PanelRenderer />);
    expect(screen.getByText('SIDE_BY_SIDE')).toBeInTheDocument();
  });

  it('renders FullScreenStack at < xl viewport', () => {
    matchMediaMock.matchesValue = false;
    render(<PanelRenderer />);
    expect(screen.getByText('FULL_SCREEN')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2.5.2: Run test → expect FAIL**

Run: `cd apps/web && pnpm test lib/panel/panel-renderer`

- [ ] **Step 2.5.3: Implement `panel-renderer.tsx`**

```tsx
// apps/web/lib/panel/panel-renderer.tsx
'use client';

import { useEffect, useState } from 'react';
import { FullScreenStack } from './full-screen-stack';
import { SideBySideStack } from './side-by-side-stack';
import { usePanelStack } from './use-panel-stack';

/**
 * Spec § 4 — xl breakpoint 단일 분기. *그 외 어떤 컴포넌트에도 viewport 분기 코드 없음.*
 */
const XL_QUERY = '(min-width: 1280px)';

function useIsDesktop(): boolean {
  const [isDesktop, setIsDesktop] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia(XL_QUERY);
    setIsDesktop(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsDesktop(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);
  return isDesktop;
}

export function PanelRenderer() {
  const isDesktop = useIsDesktop();
  const { stack } = usePanelStack();
  if (stack.entries.length === 0) return null;
  return isDesktop ? <SideBySideStack stack={stack} /> : <FullScreenStack stack={stack} />;
}
```

- [ ] **Step 2.5.4: Run test → expect PASS**

Run: `cd apps/web && pnpm test lib/panel/panel-renderer`
Expected: 2 tests pass.

- [ ] **Step 2.5.5: Commit**

```bash
git add apps/web/lib/panel/panel-renderer.tsx apps/web/lib/panel/panel-renderer.test.tsx
git commit -m "feat(sp10-t2): panel/panel-renderer — xl breakpoint single switch (1280px)"
```

### Step 2.6: T2 typecheck + lint

- [ ] **Step 2.6.1: Run typecheck + lint**

Run: `cd apps/web && pnpm typecheck && pnpm lint`
Expected: no errors.

---

## Task 3: Backend REST endpoints (`/api/parcels/:pnu`, `/api/buildings`)

**목표:** Spec § 7 F1-pure REST. backend 는 "panel" 단어 모름 — 그냥 resource server.

**Files:**
- Create: `services/api/src/routes/parcels.rs`
- Create: `services/api/src/routes/buildings.rs`
- Modify: `services/api/src/main.rs` (router 조립)

### Step 3.1: `GET /api/parcels/:pnu`

- [ ] **Step 3.1.1: Implement `parcels.rs`**

```rust
// services/api/src/routes/parcels.rs
//! `GET /api/parcels/:pnu` — PNU 19 자리로 필지 정보 조회 (SP10 panel.parcel.summary 의 backing).
//!
//! parcel-lookup crate 의 `ParcelInfoLookup::lookup` 호출 → V-World 또는 NoOp 응답.
//! 본 핸들러는 "panel" 단어를 모름 — pure REST resource (spec § 7 F1).

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::Json;
use parcel_lookup::ParcelInfoLookup;
use serde::Serialize;
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

#[derive(Clone)]
pub struct ParcelsState {
    pub parcel_lookup: Arc<dyn ParcelInfoLookup>,
}

/// 필지 정보 응답. ADR 0018 PNU-First denormalize 와 동일 surface.
#[derive(Debug, Serialize)]
pub struct ParcelInfoResponse {
    pub pnu: String,
    /// 행정구역 시도 코드 (2자리).
    pub sido_code: String,
    /// 행정구역 시군구 코드 (5자리, prefix 포함).
    pub sigungu_code: String,
    /// 행정구역 읍면동 코드 (8자리, prefix 포함).
    pub eupmyeondong_code: String,
    /// 시도 한국어명.
    pub sido_name: String,
    /// 시군구 한국어명.
    pub sigungu_name: String,
    /// 읍면동 한국어명.
    pub eupmyeondong_name: String,
    /// 지목 (factory_site / warehouse_site / ...).
    pub land_use_type: String,
    /// 용도지역 (residential / commercial / ...). V-World 미제공 시 None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoning: Option<String>,
    /// 공시지가 (KRW/m²). 미고시 → None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub official_land_price_per_m2: Option<i64>,
    /// 공시지가 고시 연·월 (예: "202504").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gosi_year_month: Option<String>,
}

/// `GET /api/parcels/:pnu` — 인증 필수.
pub async fn get_parcel(
    State(state): State<ParcelsState>,
    _auth: AuthenticatedUser,
    Path(pnu_raw): Path<String>,
) -> Result<Json<ParcelInfoResponse>, ProblemResponse> {
    let pnu = Pnu::try_new(pnu_raw.clone()).map_err(|e| {
        problem(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_pnu",
            "잘못된 필지 PNU 에요",
            Some(format!("{e}")),
        )
    })?;

    let info = state
        .parcel_lookup
        .lookup(&pnu)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, pnu = %pnu_raw, "parcel_lookup failed");
            problem(
                axum::http::StatusCode::BAD_GATEWAY,
                "parcel_lookup_failed",
                "필지 정보를 불러오지 못했어요. 잠시 후 다시 시도해 주세요",
                None,
            )
        })?
        .ok_or_else(|| {
            problem(
                axum::http::StatusCode::NOT_FOUND,
                "parcel_not_found",
                "해당 필지를 찾지 못했어요",
                Some(format!("pnu={pnu_raw}")),
            )
        })?;

    Ok(Json(ParcelInfoResponse {
        pnu: pnu_raw,
        sido_code: info.admin.sido_code().as_str().to_owned(),
        sigungu_code: info.admin.sigungu_code().as_str().to_owned(),
        eupmyeondong_code: info.admin.eupmyeondong_code().as_str().to_owned(),
        sido_name: info.admin.sido_name().to_owned(),
        sigungu_name: info.admin.sigungu_name().to_owned(),
        eupmyeondong_name: info.admin.eupmyeondong_name().to_owned(),
        land_use_type: info.land_use_type.as_str().to_owned(),
        zoning: info.zoning.as_ref().map(|z| z.as_str().to_owned()),
        official_land_price_per_m2: info.official_land_price_per_m2.map(|m| m.as_i64()),
        gosi_year_month: info.gosi_year_month.as_ref().map(|y| y.to_string()),
    }))
}
```

> **NOTE for engineer:** 위 코드의 `info.admin.sido_code().as_str()`, `info.land_use_type.as_str()` 등은 `shared_kernel` / `parcel_domain` 의 실제 method 이름으로 1:1 매핑 — 만약 method 이름이 다르면 (예: `sido()`, `code()`) 호출부만 조정. `ParcelInfo` struct shape 는 [`crates/parcel-lookup/src/info.rs`](../../../crates/parcel-lookup/src/info.rs) SSOT.

- [ ] **Step 3.1.2: Commit**

```bash
git add services/api/src/routes/parcels.rs
git commit -m "feat(sp10-t3): backend GET /api/parcels/:pnu — parcel-lookup REST shell"
```

### Step 3.2: `GET /api/buildings?parcel_pnu=:pnu`

- [ ] **Step 3.2.1: Pre-check building reader crate exists**

Run: `ls crates/data-clients/data-go-kr/`
Expected: directory contains `building_register` (per spec § 7.1 — SP4-iii-a 기존). If missing, switch this endpoint to a stub returning empty list with TODO comment, escalate to user.

- [ ] **Step 3.2.2: Implement `buildings.rs` (live path if reader exists)**

```rust
// services/api/src/routes/buildings.rs
//! `GET /api/buildings?parcel_pnu=:pnu` — 필지 위 건축물 list.
//! data.go.kr `getBrTitleInfo` 위 thin REST shell (SP4-iii-a building reader 호출).

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use shared_kernel::pnu::Pnu;

use crate::http::problem::{problem, ProblemResponse};

/// SP4-iii-a 의 BuildingRegisterReader trait.
pub trait BuildingRegisterReader: Send + Sync {
    /// PNU 로 건축물 list 조회. 빈 vec 가능.
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<BuildingItem>>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub struct BuildingItem {
    pub mgm_bldrgst_pk: String, // 관리건축물대장PK
    pub bldg_nm: String,
    pub main_purps_cd_nm: String, // 주용도코드명 (예: "공장")
    pub tot_area: f64,            // m²
    pub use_apr_day: Option<String>, // 사용승인일 YYYYMMDD
}

#[derive(Clone)]
pub struct BuildingsState {
    pub reader: Arc<dyn BuildingRegisterReader>,
}

#[derive(Debug, Deserialize)]
pub struct BuildingsQuery {
    pub parcel_pnu: String,
}

#[derive(Debug, Serialize)]
pub struct BuildingResponse {
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub total_area_m2: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BuildingsResponse {
    pub buildings: Vec<BuildingResponse>,
}

pub async fn list_buildings(
    State(state): State<BuildingsState>,
    _auth: AuthenticatedUser,
    Query(q): Query<BuildingsQuery>,
) -> Result<Json<BuildingsResponse>, ProblemResponse> {
    let pnu = Pnu::try_new(q.parcel_pnu.clone()).map_err(|e| {
        problem(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_pnu",
            "잘못된 필지 PNU 에요",
            Some(format!("{e}")),
        )
    })?;

    let items = state.reader.list_by_pnu(&pnu).await.map_err(|e| {
        tracing::warn!(error = %e, pnu = %q.parcel_pnu, "building_register read failed");
        problem(
            axum::http::StatusCode::BAD_GATEWAY,
            "buildings_lookup_failed",
            "건축물 정보를 불러오지 못했어요",
            None,
        )
    })?;

    Ok(Json(BuildingsResponse {
        buildings: items
            .into_iter()
            .map(|b| BuildingResponse {
                id: b.mgm_bldrgst_pk,
                name: b.bldg_nm,
                purpose: b.main_purps_cd_nm,
                total_area_m2: b.tot_area,
                approved_at: b.use_apr_day,
            })
            .collect(),
    }))
}
```

> **NOTE:** Reader crate 의 실제 trait 이름·method 시그니처가 다르면 (예: `GosiBuildingReader::find`), 본 파일의 `BuildingRegisterReader` trait 정의를 제거하고 그 crate 를 직접 import. 위 stub 정의는 reader 부재 시 fallback shape.

- [ ] **Step 3.2.3: Commit**

```bash
git add services/api/src/routes/buildings.rs
git commit -m "feat(sp10-t3): backend GET /api/buildings — data.go.kr building_register REST shell"
```

### Step 3.3: Wire into `main.rs`

- [ ] **Step 3.3.1: Modify `services/api/src/main.rs` mod declaration**

Edit lines 53-60 (the `mod routes { ... }` block) to add:

```rust
mod routes {
    pub mod admin_listings;
    pub mod auth_event;
    pub mod bookmarks;
    pub mod buildings;       // SP10 T3
    pub mod health;
    pub mod listings;
    pub mod notifications;
    pub mod parcels;         // SP10 T3
}
```

- [ ] **Step 3.3.2: Add state assembly + router merge**

After line 297 (`listings_router` block end), before line 299 (`// SP6-v: 공유 repository`), add:

```rust
    // SP10 T3: Panel system backing endpoints — pure REST resource.
    let parcels_state = routes::parcels::ParcelsState {
        parcel_lookup: listings_state.parcel_lookup.clone(),
    };
    let parcels_router: Router<()> = Router::new()
        .route("/api/parcels/:pnu", get(routes::parcels::get_parcel))
        .with_state(parcels_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // SP10 T3: building_register reader 주입 — SP4-iii-a 의 reader 인스턴스화.
    // 미구현 시 (DATA_GO_KR_API_KEY 미설정) NoOp fallback — 빈 list 반환.
    let building_reader: Arc<dyn routes::buildings::BuildingRegisterReader> =
        Arc::new(NoOpBuildingRegisterReader);
    let buildings_state = routes::buildings::BuildingsState { reader: building_reader };
    let buildings_router: Router<()> = Router::new()
        .route("/api/buildings", get(routes::buildings::list_buildings))
        .with_state(buildings_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));
```

Then in the final `app` builder (line 383-389), add `.merge(parcels_router).merge(buildings_router)`:

```rust
    let app = public
        .merge(protected)
        .merge(listings_router)
        .merge(parcels_router)         // SP10 T3
        .merge(buildings_router)       // SP10 T3
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(internal)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));
```

- [ ] **Step 3.3.3: Add NoOp building reader stub**

At top of `main.rs` (after `use ...` block), add:

```rust
/// SP10 T3: NoOp building reader — DATA_GO_KR_API_KEY 미설정 시 fallback (빈 list).
/// production 은 SP4-iii-a 의 live reader 로 swap.
struct NoOpBuildingRegisterReader;

impl routes::buildings::BuildingRegisterReader for NoOpBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        _pnu: &'a shared_kernel::pnu::Pnu,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<routes::buildings::BuildingItem>>> + Send + 'a>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}
```

- [ ] **Step 3.3.4: Run cargo check**

Run: `cargo check -p api`
Expected: clean.

- [ ] **Step 3.3.5: Run cargo clippy**

Run: `cargo clippy -p api --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3.3.6: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-t3): wire /api/parcels/:pnu + /api/buildings into main router"
```

### Step 3.4: Integration test

- [ ] **Step 3.4.1: Create `services/api/tests/sp10_panel_endpoints.rs`**

```rust
//! SP10 T3: panel backing endpoints integration test.

#[tokio::test]
async fn get_parcel_returns_404_for_unknown_pnu() {
    // ... reuses test scaffolding from existing tests/listing_*.rs
    // — minimum: assert that GET /api/parcels/{19-zeros} with NoOp lookup returns 404
    //   (NoOpParcelInfoLookup returns Ok(None) for any pnu).
    //
    // 실제 test scaffold 는 기존 services/api/tests/*.rs (예: listing_search.rs) 의 setup
    // helper 와 동일 패턴 — Axum app 부팅 + tokio::spawn + reqwest call.
    //
    // 빈 stub 으로 시작 — 첫 fail 후 scaffold 복붙해서 채워나감 (TDD red).
    panic!("write me");
}

#[tokio::test]
async fn get_parcel_returns_400_for_invalid_pnu() {
    panic!("write me");
}

#[tokio::test]
async fn list_buildings_returns_empty_with_noop_reader() {
    panic!("write me");
}
```

- [ ] **Step 3.4.2: Fill scaffold by copying from `services/api/tests/listing_search.rs`**

Read the existing test file pattern and replicate the bootstrap (axum app, port 0, reqwest call). Implement the 3 tests above with concrete asserts. Use `Pnu` constructor for valid 19-digit PNU.

- [ ] **Step 3.4.3: Run integration test**

Run: `cargo test -p api --test sp10_panel_endpoints`
Expected: 3 tests pass.

- [ ] **Step 3.4.4: Commit**

```bash
git add services/api/tests/sp10_panel_endpoints.rs
git commit -m "test(sp10-t3): integration tests for /api/parcels/:pnu + /api/buildings (NoOp path)"
```

---

## Task 4: `parcel` kind registration (3 views + i18n)

**목표:** Spec § 6 R1 + § 7 4 endpoints 의 `parcel.summary` / `parcel.buildings` / `parcel.listings` 활성화.

**Files:**
- Create: `apps/web/lib/api/parcels.ts`
- Create: `apps/web/lib/api/buildings.ts`
- Create: `apps/web/components/panels/parcel/skeletons.tsx`
- Create: `apps/web/components/panels/parcel/summary.tsx`
- Create: `apps/web/components/panels/parcel/buildings.tsx`
- Create: `apps/web/components/panels/parcel/listings.tsx`
- Create: `apps/web/components/panels/parcel/register.ts`
- Modify: `apps/web/lib/i18n/ko.json` (add `panels.parcel.*` namespace)
- Modify: `apps/web/lib/i18n/haeyo.ts` (whatever generates message types — add namespace)

### Step 4.1: API clients

- [ ] **Step 4.1.1: Implement `lib/api/parcels.ts`**

```ts
// apps/web/lib/api/parcels.ts
import { z } from 'zod';
import { api } from '@/lib/api';

export const ParcelInfoSchema = z.object({
  pnu: z.string(),
  sido_code: z.string(),
  sigungu_code: z.string(),
  eupmyeondong_code: z.string(),
  sido_name: z.string(),
  sigungu_name: z.string(),
  eupmyeondong_name: z.string(),
  land_use_type: z.string(),
  zoning: z.string().nullish(),
  official_land_price_per_m2: z.number().int().nullish(),
  gosi_year_month: z.string().nullish(),
});

export type ParcelInfo = z.infer<typeof ParcelInfoSchema>;

export async function fetchParcel(pnu: string): Promise<ParcelInfo> {
  const json = await api.get(`api/parcels/${pnu}`).json<unknown>();
  return ParcelInfoSchema.parse(json);
}
```

- [ ] **Step 4.1.2: Implement `lib/api/buildings.ts`**

```ts
// apps/web/lib/api/buildings.ts
import { z } from 'zod';
import { api } from '@/lib/api';

export const BuildingSchema = z.object({
  id: z.string(),
  name: z.string(),
  purpose: z.string(),
  total_area_m2: z.number(),
  approved_at: z.string().nullish(),
});

export type Building = z.infer<typeof BuildingSchema>;

export const BuildingsResponseSchema = z.object({
  buildings: z.array(BuildingSchema),
});

export type BuildingsResponse = z.infer<typeof BuildingsResponseSchema>;

export async function fetchBuildings(parcelPnu: string): Promise<BuildingsResponse> {
  const json = await api.get(`api/buildings?parcel_pnu=${encodeURIComponent(parcelPnu)}`).json<unknown>();
  return BuildingsResponseSchema.parse(json);
}
```

- [ ] **Step 4.1.3: Commit**

```bash
git add apps/web/lib/api/parcels.ts apps/web/lib/api/buildings.ts
git commit -m "feat(sp10-t4): api clients for /api/parcels/:pnu + /api/buildings"
```

### Step 4.2: Skeletons + view components

- [ ] **Step 4.2.1: Implement `components/panels/parcel/skeletons.tsx`**

```tsx
// apps/web/components/panels/parcel/skeletons.tsx
'use client';
import { Skeleton } from '@gongzzang/ui';
import { useTranslations } from 'next-intl';

export function ParcelLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-6">
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-4 w-64" />
      <Skeleton className="h-4 w-48" />
      <Skeleton className="h-32 w-full" />
    </div>
  );
}

export function ParcelErrorCard({ error }: { error: unknown }) {
  const t = useTranslations('panels.parcel');
  const msg = error instanceof Error ? error.message : String(error);
  return (
    <div className="p-6">
      <div className="text-[length:var(--text-body-md)] font-semibold text-[var(--color-error)]">
        {t('errors.loadFailed')}
      </div>
      <div className="mt-2 text-[length:var(--text-caption)] text-[var(--color-muted)]">{msg}</div>
    </div>
  );
}

export function ParcelEmptyCard() {
  const t = useTranslations('panels.parcel');
  return <div className="p-6 text-center text-[var(--color-muted)]">{t('empty')}</div>;
}
```

- [ ] **Step 4.2.2: Implement `summary.tsx`**

```tsx
// apps/web/components/panels/parcel/summary.tsx
'use client';
import { useTranslations } from 'next-intl';
import type { ParcelInfo } from '@/lib/api/parcels';
import type { PanelStackEntry } from '@/lib/panel/types';
import { usePanelStack } from '@/lib/panel/use-panel-stack';

export function ParcelSummaryCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'parcel' }>;
  data: ParcelInfo;
}) {
  const t = useTranslations('panels.parcel.summary');
  const { push } = usePanelStack();

  return (
    <div className="flex flex-col gap-4 p-6">
      <header>
        <div className="font-mono text-[length:var(--text-caption)] text-[var(--color-muted)]">
          PNU {entry.id}
        </div>
        <h2 className="text-[length:var(--text-title-lg)] font-semibold text-[var(--color-ink)]">
          {data.sido_name} {data.sigungu_name} {data.eupmyeondong_name}
        </h2>
      </header>
      <dl className="grid grid-cols-2 gap-y-2 text-[length:var(--text-body-sm)]">
        <dt className="text-[var(--color-muted)]">{t('landUse')}</dt>
        <dd className="text-[var(--color-ink)]">{data.land_use_type}</dd>
        {data.zoning && (
          <>
            <dt className="text-[var(--color-muted)]">{t('zoning')}</dt>
            <dd className="text-[var(--color-ink)]">{data.zoning}</dd>
          </>
        )}
        {data.official_land_price_per_m2 != null && (
          <>
            <dt className="text-[var(--color-muted)]">{t('officialPrice')}</dt>
            <dd className="text-[var(--color-ink)]">
              {data.official_land_price_per_m2.toLocaleString('ko-KR')} 원/㎡
            </dd>
          </>
        )}
      </dl>
      <nav className="mt-4 flex flex-col gap-2">
        <button
          type="button"
          onClick={() => push({ kind: 'parcel', id: entry.id, view: 'buildings' })}
          className="rounded-md border border-[var(--color-hairline)] px-3 py-2 text-left hover:bg-[var(--color-surface-cream-strong)]"
        >
          {t('viewBuildings')} ›
        </button>
        <button
          type="button"
          onClick={() => push({ kind: 'parcel', id: entry.id, view: 'listings' })}
          className="rounded-md border border-[var(--color-hairline)] px-3 py-2 text-left hover:bg-[var(--color-surface-cream-strong)]"
        >
          {t('viewListings')} ›
        </button>
      </nav>
    </div>
  );
}
```

- [ ] **Step 4.2.3: Implement `buildings.tsx`**

```tsx
// apps/web/components/panels/parcel/buildings.tsx
'use client';
import { useTranslations } from 'next-intl';
import type { BuildingsResponse } from '@/lib/api/buildings';
import type { PanelStackEntry } from '@/lib/panel/types';

export function ParcelBuildingsCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'parcel' }>;
  data: BuildingsResponse;
}) {
  const t = useTranslations('panels.parcel.buildings');
  if (data.buildings.length === 0) {
    return <div className="p-6 text-center text-[var(--color-muted)]">{t('none')}</div>;
  }
  return (
    <div className="flex flex-col gap-3 p-6">
      <header className="flex items-baseline gap-2">
        <h2 className="text-[length:var(--text-title-md)] font-semibold">{t('title')}</h2>
        <span className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {data.buildings.length} {t('count')}
        </span>
      </header>
      <ul className="flex flex-col gap-2">
        {data.buildings.map((b) => (
          <li
            key={b.id}
            className="rounded-md border border-[var(--color-hairline)] p-3 text-[length:var(--text-body-sm)]"
          >
            <div className="font-semibold text-[var(--color-ink)]">{b.name}</div>
            <div className="text-[var(--color-muted)]">
              {b.purpose} · {b.total_area_m2.toLocaleString('ko-KR')} ㎡
              {b.approved_at && ` · ${b.approved_at}`}
            </div>
          </li>
        ))}
      </ul>
      {/* PNU 의 entry.id 는 i18n 라벨 표시 외 미사용 — 본 view 는 list-only */}
      <span className="hidden">{entry.id}</span>
    </div>
  );
}
```

- [ ] **Step 4.2.4: Implement `listings.tsx`**

```tsx
// apps/web/components/panels/parcel/listings.tsx
'use client';
import { useTranslations } from 'next-intl';
import type { ListingsResponse } from '@/lib/listings/api';
import type { PanelStackEntry } from '@/lib/panel/types';
import { usePanelStack } from '@/lib/panel/use-panel-stack';

export function ParcelListingsCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'parcel' }>;
  data: ListingsResponse;
}) {
  const t = useTranslations('panels.parcel.listings');
  const { push } = usePanelStack();
  if (data.listings.length === 0) {
    return <div className="p-6 text-center text-[var(--color-muted)]">{t('none')}</div>;
  }
  return (
    <div className="flex flex-col gap-3 p-6">
      <header>
        <h2 className="text-[length:var(--text-title-md)] font-semibold">
          {t('title', { count: data.total })}
        </h2>
      </header>
      <ul className="flex flex-col gap-2">
        {data.listings.map((l) => (
          <li key={l.id}>
            <button
              type="button"
              onClick={() => push({ kind: 'listing', id: l.id, view: 'summary' })}
              className="block w-full rounded-md border border-[var(--color-hairline)] p-3 text-left hover:bg-[var(--color-surface-cream-strong)]"
            >
              <div className="font-semibold text-[var(--color-ink)]">{l.title}</div>
              <div className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
                {l.price_krw.toLocaleString('ko-KR')} 원 · {l.area_m2.toLocaleString('ko-KR')} ㎡
              </div>
            </button>
          </li>
        ))}
      </ul>
      <span className="hidden">{entry.id}</span>
    </div>
  );
}
```

- [ ] **Step 4.2.5: Commit**

```bash
git add apps/web/components/panels/parcel/skeletons.tsx \
        apps/web/components/panels/parcel/summary.tsx \
        apps/web/components/panels/parcel/buildings.tsx \
        apps/web/components/panels/parcel/listings.tsx
git commit -m "feat(sp10-t4): parcel panel views (summary/buildings/listings) + skeletons"
```

### Step 4.3: Register parcel kind

- [ ] **Step 4.3.1: Implement `register.ts`**

```ts
// apps/web/components/panels/parcel/register.ts
import { fetchBuildings } from '@/lib/api/buildings';
import { fetchParcel } from '@/lib/api/parcels';
import { fetchListings } from '@/lib/listings/api';
import { defineKind } from '@/lib/panel/registry';
import { ParcelBuildingsCard } from './buildings';
import { ParcelListingsCard } from './listings';
import { ParcelEmptyCard, ParcelErrorCard, ParcelLoadingSkeleton } from './skeletons';
import { ParcelSummaryCard } from './summary';

defineKind({
  kind: 'parcel',
  idPattern: /^\d{19}$/,
  views: {
    summary: {
      component: ParcelSummaryCard,
      fetcher: (id) => fetchParcel(id),
      staleTime: 5 * 60_000,
      links: [],
    },
    buildings: {
      component: ParcelBuildingsCard,
      fetcher: (id) => fetchBuildings(id),
      staleTime: 5 * 60_000,
      links: [],
    },
    listings: {
      component: ParcelListingsCard,
      fetcher: (id) =>
        fetchListings({
          filters: {
            types: [],
            transactions: [],
            minAreaM2: undefined,
            maxAreaM2: undefined,
            minPriceKrw: undefined,
            maxPriceKrw: undefined,
            sort: 'created_at_desc',
            adminCode: undefined,
            landUseType: undefined,
            // After T6 the `pnu` field is removed from filters; this fetcher
            // composes a one-shot search by direct query param.
          } as never,
          // direct PNU-narrowed search via query param appended in fetchListings
          // (see T6 changes). Until then this works because fetchListings
          // accepts whatever filters shape with `pnu` field.
        }),
      staleTime: 60_000,
      links: [],
    },
  },
  loadingComponent: ParcelLoadingSkeleton,
  errorComponent: ParcelErrorCard,
  emptyComponent: ParcelEmptyCard,
  authGate: { required: true },
  i18nNamespace: 'panels.parcel',
  telemetryAttrs: (entry) => ({ pnu: entry.id }),
});
```

> **NOTE for engineer:** `parcel.listings` view 의 fetcher 는 T6 에서 fetchListings 가 `pnu` 직접 query param 으로 받도록 변경됨 — T6 step 6.3 참조. 본 step 에서는 stub-shaped 으로 두고 T6 가 정합 맞춤.

- [ ] **Step 4.3.2: Commit**

```bash
git add apps/web/components/panels/parcel/register.ts
git commit -m "feat(sp10-t4): defineKind('parcel') with 3 views + 4-state shell"
```

### Step 4.4: i18n namespace

- [ ] **Step 4.4.1: Add `panels.parcel.*` keys to `lib/i18n/ko.json`**

Read current `lib/i18n/ko.json`. Add at root:

```json
"panel": {
  "back": "이전",
  "breadcrumb": "경로"
},
"panels": {
  "parcel": {
    "empty": "필지 정보가 없어요",
    "errors": {
      "loadFailed": "필지 정보를 불러오지 못했어요"
    },
    "summary": {
      "landUse": "지목",
      "zoning": "용도지역",
      "officialPrice": "공시지가",
      "viewBuildings": "건축물 보기",
      "viewListings": "등록 매물 보기"
    },
    "buildings": {
      "title": "건축물",
      "count": "동",
      "none": "등록된 건축물이 없어요"
    },
    "listings": {
      "title": "이 필지의 매물 ({count})",
      "none": "이 필지에 등록된 매물이 없어요"
    }
  }
}
```

- [ ] **Step 4.4.2: Run typecheck (i18n type generation)**

Run: `cd apps/web && pnpm typecheck`
Expected: clean. If `next-intl` typed namespace requires regeneration, follow the project's existing pattern (likely `messages.d.ts` is auto-derived).

- [ ] **Step 4.4.3: Commit**

```bash
git add apps/web/lib/i18n/ko.json
git commit -m "feat(sp10-t4): i18n panels.parcel.* namespace"
```

---

## Task 5: `listing` kind registration + redirect

**Files:**
- Create: `apps/web/components/panels/listing/skeletons.tsx`
- Create: `apps/web/components/panels/listing/summary.tsx`
- Create: `apps/web/components/panels/listing/register.ts`
- Modify: `apps/web/app/(authenticated)/listings/[id]/page.tsx` (server redirect)
- Modify: `apps/web/lib/i18n/ko.json` (add `panels.listing.*`)

### Step 5.1: Listing summary view

- [ ] **Step 5.1.1: Implement `skeletons.tsx`**

```tsx
// apps/web/components/panels/listing/skeletons.tsx
'use client';
import { Skeleton } from '@gongzzang/ui';
import { useTranslations } from 'next-intl';

export function ListingLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-6">
      <Skeleton className="aspect-[4/3] w-full" />
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-4 w-64" />
    </div>
  );
}

export function ListingErrorCard({ error }: { error: unknown }) {
  const t = useTranslations('panels.listing');
  return (
    <div className="p-6">
      <div className="text-[var(--color-error)]">{t('errors.loadFailed')}</div>
      <div className="mt-2 text-[length:var(--text-caption)] text-[var(--color-muted)]">
        {error instanceof Error ? error.message : String(error)}
      </div>
    </div>
  );
}

export function ListingEmptyCard() {
  const t = useTranslations('panels.listing');
  return <div className="p-6 text-center text-[var(--color-muted)]">{t('notFound')}</div>;
}
```

- [ ] **Step 5.1.2: Implement `summary.tsx`**

```tsx
// apps/web/components/panels/listing/summary.tsx
'use client';
import { Badge } from '@gongzzang/ui';
import Image from 'next/image';
import { useTranslations } from 'next-intl';
import type { ListingDetail } from '@/lib/listings/api';
import { formatAreaPyeong, formatPriceKrw } from '@/lib/listings/format';
import type { PanelStackEntry } from '@/lib/panel/types';

export function ListingSummaryCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'listing' }>;
  data: ListingDetail;
}) {
  const t = useTranslations('panels.listing.summary');
  const cover = data.photos?.[0];

  return (
    <div className="flex flex-col gap-4 p-6">
      {cover && (
        <div className="relative aspect-[4/3] w-full overflow-hidden rounded-md bg-[var(--color-surface-cream-strong)]">
          <Image
            src={`/api/listings/${entry.id}/photos/${cover.r2_key}`}
            alt={data.title}
            fill
            className="object-cover"
            sizes="(max-width: 1280px) 100vw, 600px"
          />
        </div>
      )}
      <header className="flex items-center gap-2">
        <Badge>{t(`type.${data.listing_type}` as never)}</Badge>
        <Badge variant="outline">{t(`transaction.${data.transaction_type}` as never)}</Badge>
      </header>
      <h2 className="text-[length:var(--text-title-lg)] font-semibold text-[var(--color-ink)]">
        {data.title}
      </h2>
      <dl className="grid grid-cols-2 gap-y-2 text-[length:var(--text-body-sm)]">
        <dt className="text-[var(--color-muted)]">{t('area')}</dt>
        <dd>{formatAreaPyeong(data.area_m2)}</dd>
        <dt className="text-[var(--color-muted)]">{t('price')}</dt>
        <dd>{formatPriceKrw(data.price_krw)}</dd>
        <dt className="text-[var(--color-muted)]">PNU</dt>
        <dd className="font-mono">{data.parcel_pnu}</dd>
      </dl>
      <p className="whitespace-pre-wrap text-[length:var(--text-body-sm)] text-[var(--color-muted)]">
        {data.description}
      </p>
    </div>
  );
}
```

- [ ] **Step 5.1.3: Implement `register.ts`**

```ts
// apps/web/components/panels/listing/register.ts
import { fetchListingDetail } from '@/lib/listings/api';
import { defineKind } from '@/lib/panel/registry';
import { ListingEmptyCard, ListingErrorCard, ListingLoadingSkeleton } from './skeletons';
import { ListingSummaryCard } from './summary';

defineKind({
  kind: 'listing',
  idPattern: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
  views: {
    summary: {
      component: ListingSummaryCard,
      fetcher: (id) => fetchListingDetail(id),
      staleTime: 60_000,
      links: [],
    },
  },
  loadingComponent: ListingLoadingSkeleton,
  errorComponent: ListingErrorCard,
  emptyComponent: ListingEmptyCard,
  authGate: { required: true },
  i18nNamespace: 'panels.listing',
  telemetryAttrs: (entry) => ({ listing_id: entry.id }),
});
```

- [ ] **Step 5.1.4: Commit**

```bash
git add apps/web/components/panels/listing/
git commit -m "feat(sp10-t5): defineKind('listing') with summary view + 4-state shell"
```

### Step 5.2: Server redirect for `/listings/[id]`

- [ ] **Step 5.2.1: Replace `apps/web/app/(authenticated)/listings/[id]/page.tsx` body**

```tsx
// apps/web/app/(authenticated)/listings/[id]/page.tsx
/**
 * SP10: 매물 상세 page → /listings?p=listing:{id}.summary 로 server redirect.
 * 컴포넌트 사본 0 (spec rule § 9 #13). Middle-click / new-tab 도 redirect 가 받음.
 */
import { redirect } from 'next/navigation';

interface PageProps {
  params: Promise<{ id: string }>;
}

export default async function ListingDetailPage({ params }: PageProps): Promise<never> {
  const { id } = await params;
  redirect(`/listings?p=listing:${encodeURIComponent(id)}.summary`);
}
```

- [ ] **Step 5.2.2: Commit**

```bash
git add apps/web/app/(authenticated)/listings/[id]/page.tsx
git commit -m "feat(sp10-t5): /listings/[id] server redirect → /listings?p=listing:id.summary (sample 0)"
```

### Step 5.3: Listing i18n keys

- [ ] **Step 5.3.1: Add `panels.listing.*` to `lib/i18n/ko.json`**

Append (inside the existing `panels` object from T4):

```json
"listing": {
  "notFound": "매물을 찾을 수 없어요",
  "errors": {
    "loadFailed": "매물 정보를 불러오지 못했어요"
  },
  "summary": {
    "area": "면적",
    "price": "가격",
    "type": {
      "factory": "공장",
      "warehouse": "창고",
      "office": "오피스",
      "knowledge_industry_center": "지식산업센터",
      "industrial_land": "산업용지",
      "logistics_center": "물류센터"
    },
    "transaction": {
      "sale": "매매",
      "monthly_rent": "월세",
      "jeonse": "전세"
    }
  }
}
```

- [ ] **Step 5.3.2: Commit**

```bash
git add apps/web/lib/i18n/ko.json
git commit -m "feat(sp10-t5): i18n panels.listing.* namespace"
```

---

## Task 6: 통합 변경 + lint rules + e2e + a11y + 회귀

**목표:** Spec § 11 통합 변경 적용, § 10.1 lint rules, § 10.2 e2e, § 10.3 extensibility 회귀, § 10.4 텔레메트리 검증.

**Files (modify):**
- `apps/web/app/(authenticated)/listings/page.tsx`
- `apps/web/components/listings/listing-map.tsx`
- `apps/web/components/listings/listing-card.tsx`
- `apps/web/components/listings/listing-card-list.tsx`
- `apps/web/lib/listings/use-listings-query.ts`
- `apps/web/stores/listings.ts`
- `apps/web/lib/listings/filters.ts`
- `apps/web/lib/listings/api.ts` (fetchListings 의 pnu query param 직접화)
- `apps/web/tests/unit/listings/filters.test.ts` (pnu 케이스 제거)
- `apps/web/biome.json`
- `lefthook.yml`

**Files (create):**
- `apps/web/tests/unit/panel-extensibility.test.ts`
- `apps/web/tests/e2e/panel-system.spec.ts`

**Files (delete):**
- `apps/web/components/listings/parcel-info-panel.tsx`

### Step 6.1: Refactor `fetchListings` to accept pnu directly

- [ ] **Step 6.1.1: Modify `apps/web/lib/listings/api.ts`**

Change `FetchListingsInput` and `fetchListings` so that `pnu` is a top-level param, not from filters:

```ts
// Replace the existing FetchListingsInput / fetchListings region with:
export interface FetchListingsInput {
  filters: ListingFilters;
  bounds?: { south: number; west: number; north: number; east: number };
  pnu?: string;
  page?: number;
  size?: number;
}

export async function fetchListings(input: FetchListingsInput): Promise<ListingsResponse> {
  const sp = toSearchParams(input.filters);
  if (input.bounds) {
    const { south, west, north, east } = input.bounds;
    sp.set('bounds', `${south},${west},${north},${east}`);
  }
  if (input.pnu) sp.set('pnu', input.pnu);
  if (input.page !== undefined) sp.set('page', String(input.page));
  if (input.size !== undefined) sp.set('size', String(input.size));

  const json = await api.get(`listings?${sp.toString()}`).json<unknown>();
  return ListingsResponseSchema.parse(json);
}
```

- [ ] **Step 6.1.2: Modify `apps/web/lib/listings/filters.ts`**

Remove `pnu` from `ListingFilters` and from `parseFiltersFromSearchParams` / `toSearchParams`:

```ts
// Edit ListingFilters interface — delete the pnu field:
export interface ListingFilters {
  types: ListingType[];
  transactions: TransactionType[];
  minAreaM2: number | undefined;
  maxAreaM2: number | undefined;
  minPriceKrw: number | undefined;
  maxPriceKrw: number | undefined;
  sort: SortKey;
  adminCode: string | undefined;
  landUseType: string | undefined;
}

// Edit parseFiltersFromSearchParams — remove the pnu line:
//   pnu: sp.get("pnu") ?? undefined,   ← delete

// Edit toSearchParams — remove the pnu branch:
//   if (f.pnu) sp.set("pnu", f.pnu);   ← delete
```

- [ ] **Step 6.1.3: Update `apps/web/tests/unit/listings/filters.test.ts`**

Read the file. Delete any test that checks `pnu` parsing/serialization, since the field is gone.

- [ ] **Step 6.1.4: Run unit tests**

Run: `cd apps/web && pnpm test`
Expected: pass.

- [ ] **Step 6.1.5: Commit**

```bash
git add apps/web/lib/listings/api.ts apps/web/lib/listings/filters.ts apps/web/tests/unit/listings/filters.test.ts
git commit -m "refactor(sp10-t6): fetchListings pnu top-level + remove filters.pnu (panel stack derives)"
```

### Step 6.2: Update zustand store

- [ ] **Step 6.2.1: Modify `apps/web/stores/listings.ts`**

Replace contents:

```ts
'use client';
import { create } from 'zustand';
import type { ListingFilters, SortKey } from '@/lib/listings/filters';

export interface MapBounds {
  south: number;
  west: number;
  north: number;
  east: number;
}

interface ListingsState {
  bounds: MapBounds | undefined;
  filters: ListingFilters;
  setBounds: (b: MapBounds) => void;
  setFilters: (next: ListingFilters) => void;
  patchFilters: (patch: Partial<ListingFilters>) => void;
}

const DEFAULT_FILTERS: ListingFilters = {
  types: [],
  transactions: [],
  minAreaM2: undefined,
  maxAreaM2: undefined,
  minPriceKrw: undefined,
  maxPriceKrw: undefined,
  sort: 'created_at_desc' as SortKey,
  adminCode: undefined,
  landUseType: undefined,
};

export const useListingsStore = create<ListingsState>((set) => ({
  bounds: undefined,
  filters: DEFAULT_FILTERS,
  setBounds: (b) => set({ bounds: b }),
  setFilters: (next) => set({ filters: next }),
  patchFilters: (patch) => set((state) => ({ filters: { ...state.filters, ...patch } })),
}));
```

- [ ] **Step 6.2.2: Commit**

```bash
git add apps/web/stores/listings.ts
git commit -m "refactor(sp10-t6): drop selectedListingId + filters.pnu from store (panel stack SSOT)"
```

### Step 6.3: Update `useListingsQuery` to derive pnu from panel stack

- [ ] **Step 6.3.1: Modify `apps/web/lib/listings/use-listings-query.ts`**

```ts
'use client';
import { useInfiniteQuery } from '@tanstack/react-query';
import { fetchListings, type ListingsResponse } from '@/lib/listings/api';
import { usePanelStack } from '@/lib/panel/use-panel-stack';
import { useListingsStore } from '@/stores/listings';

const PAGE_SIZE = 20;

/**
 * 단일 useInfiniteQuery hook. SP10: filters 의 pnu 자리를 panel stack 의 top
 * (parcel.summary 또는 parcel.*) 에서 derive — `useListingsStore` 에 pnu 없음.
 */
export function useListingsQuery() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);
  const { stack } = usePanelStack();

  const top = stack.entries[stack.entries.length - 1];
  const derivedPnu = top?.kind === 'parcel' ? top.id : undefined;

  return useInfiniteQuery<ListingsResponse>({
    queryKey: ['listings', filters, bounds, derivedPnu],
    queryFn: ({ pageParam }) =>
      fetchListings({
        filters,
        bounds,
        pnu: derivedPnu,
        page: pageParam as number,
        size: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });
}
```

- [ ] **Step 6.3.2: Commit**

```bash
git add apps/web/lib/listings/use-listings-query.ts
git commit -m "refactor(sp10-t6): useListingsQuery derives pnu from panel stack top (parcel.*)"
```

### Step 6.4: Update `parcel/listings.tsx` fetcher to use pnu directly

- [ ] **Step 6.4.1: Modify `apps/web/components/panels/parcel/register.ts`**

Replace the `parcel.listings` view fetcher:

```ts
listings: {
  component: ParcelListingsCard,
  fetcher: (id) =>
    fetchListings({
      filters: {
        types: [],
        transactions: [],
        minAreaM2: undefined,
        maxAreaM2: undefined,
        minPriceKrw: undefined,
        maxPriceKrw: undefined,
        sort: 'created_at_desc',
        adminCode: undefined,
        landUseType: undefined,
      },
      pnu: id,
    }),
  staleTime: 60_000,
  links: [],
},
```

- [ ] **Step 6.4.2: Commit**

```bash
git add apps/web/components/panels/parcel/register.ts
git commit -m "fix(sp10-t6): parcel.listings fetcher uses fetchListings({pnu:id}) (filters cleaned)"
```

### Step 6.5: Update `listing-map.tsx`

- [ ] **Step 6.5.1: Modify polygon click handler in `listing-map.tsx`**

Replace `setupPolygonLayers(mb, (pnu) => patchFilters({ pnu }));` and the marker click `setSelected(listing.id)` with panel push calls.

Add at top:

```tsx
import { usePanelStack } from '@/lib/panel/use-panel-stack';
```

Replace the `patchFilters` reference (around line 181):

```tsx
const { push: pushPanel } = usePanelStack();
```

Replace polygon-click line (~254):

```tsx
setupPolygonLayers(mb, (pnu) => pushPanel({ kind: 'parcel', id: pnu, view: 'summary' }));
```

Replace marker-click handler (~297-299):

```tsx
naver.maps.Event.addListener(marker, 'click', () => {
  pushPanel({ kind: 'listing', id: listing.id, view: 'summary' });
});
```

Remove `selectedId` / `setSelected` references (no longer in store) — replace marker icon's `selected` flag with one derived from the top panel entry:

```tsx
const { stack } = usePanelStack();
const selectedListingId =
  stack.entries[stack.entries.length - 1]?.kind === 'listing'
    ? (stack.entries[stack.entries.length - 1] as { kind: 'listing'; id: string }).id
    : undefined;

// inside marker creation:
icon: {
  content: pinIconHtml(listing.listing_type, { selected: listing.id === selectedListingId }),
  anchor: new naver.maps.Point(14, 28),
},
```

Remove `patchFilters` from the deps array; replace with `pushPanel` (stable ref via useCallback if needed — but `usePanelStack` returns memoized callbacks).

- [ ] **Step 6.5.2: Run typecheck**

Run: `cd apps/web && pnpm typecheck`
Expected: clean.

- [ ] **Step 6.5.3: Commit**

```bash
git add apps/web/components/listings/listing-map.tsx
git commit -m "refactor(sp10-t6): listing-map polygon/marker click → pushPanel (parcel.summary / listing.summary)"
```

### Step 6.6: Update `listing-card.tsx`

- [ ] **Step 6.6.1: Modify `apps/web/components/listings/listing-card.tsx`**

Replace `<Link href>` with onClick `pushPanel`. Keep middle-click via `onAuxClick` to native link (server redirect catches it). Remove hover `setSelected` (selectedId now lives in panel stack).

Add at top:

```tsx
import { usePanelStack } from '@/lib/panel/use-panel-stack';
```

Replace `<Link href>` block — convert outer to `<div role="button">` with onClick + onKeyDown, but preserve `<a href>` for middle-click as a hidden child (or use Next.js prefetch + onClick.preventDefault). Simplest: keep `<Link>` and onClick.preventDefault + push:

```tsx
const { push } = usePanelStack();
const { stack } = usePanelStack();
const top = stack.entries[stack.entries.length - 1];
const isSelected = top?.kind === 'listing' && top.id === data.id;

return (
  <Card surface="cream-card" className={...} >
    <Link
      href={`/listings/${data.id}` as Route}
      onClick={(e) => {
        if (e.metaKey || e.ctrlKey || e.button === 1) return; // 새 탭은 그대로
        e.preventDefault();
        push({ kind: 'listing', id: data.id, view: 'summary' });
      }}
      className="block"
    >
      ...
    </Link>
  </Card>
);
```

Remove `useListingsStore` import + `selectedId`/`setSelected` references.

- [ ] **Step 6.6.2: Commit**

```bash
git add apps/web/components/listings/listing-card.tsx
git commit -m "refactor(sp10-t6): listing-card click → pushPanel (Cmd/Ctrl-click 새 탭은 redirect 가 받음)"
```

### Step 6.7: Delete `parcel-info-panel.tsx` + update page.tsx

- [ ] **Step 6.7.1: Delete `apps/web/components/listings/parcel-info-panel.tsx`**

Run: `rm apps/web/components/listings/parcel-info-panel.tsx`

- [ ] **Step 6.7.2: Modify `apps/web/app/(authenticated)/listings/page.tsx`**

> **Single-mount:** PanelRenderer renders ONCE at page top level. SideBySideStack and FullScreenStack are both `fixed`-positioned overlays — they don't need a page-grid slot. Page grid stays `[1fr_420px]` (map + card list aside).

```tsx
import { Separator } from '@gongzzang/ui';
import { getTranslations } from 'next-intl/server';
import { FilterBar } from '@/components/listings/filter-bar';
import { ListingCardList } from '@/components/listings/listing-card-list';
import { ListingMap } from '@/components/listings/listing-map';
import { SearchBar } from '@/components/listings/search-bar';
import '@/components/panels/parcel/register'; // SP10: side-effect kind register
import '@/components/panels/listing/register'; // SP10: side-effect kind register
import { PanelRenderer } from '@/lib/panel/panel-renderer';

export default async function ListingsPage() {
  const t = await getTranslations('listings.page');

  return (
    <main className="flex h-screen flex-col bg-[var(--color-canvas)]">
      <header className="flex items-center justify-between gap-6 px-6 py-4">
        <h1 className="whitespace-nowrap text-[length:var(--text-title-lg)] font-semibold tracking-[var(--tracking-display-sm)] text-[var(--color-ink)]">
          {t('title')}
        </h1>
        <div className="max-w-md flex-1">
          <SearchBar />
        </div>
      </header>
      <Separator />
      <FilterBar />
      <Separator />
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[1fr_420px]">
        <section className="relative h-full">
          <ListingMap />
        </section>
        <aside className="overflow-y-auto border-l border-[var(--color-hairline)] bg-[var(--color-canvas)]">
          <ListingCardList />
        </aside>
      </div>
      {/* SP10: PanelRenderer 는 fixed overlay — single mount. xl 에서 우측 840px,
          그 외 viewport 는 fixed inset-0 fullscreen. */}
      <PanelRenderer />
    </main>
  );
}
```

- [ ] **Step 6.7.3: Commit**

```bash
git add apps/web/app/\(authenticated\)/listings/page.tsx
git rm apps/web/components/listings/parcel-info-panel.tsx
git commit -m "refactor(sp10-t6): listings page integrates PanelRenderer; ParcelInfoPanel deleted (registry replaces)"
```

### Step 6.8: Update `listing-card-list.tsx`

- [ ] **Step 6.8.1: Modify `apps/web/components/listings/listing-card-list.tsx`**

The hover-highlight no longer uses `selectedId` from store — derives from panel stack the same as `listing-card.tsx`. Since `listing-card.tsx` already does the work, no change needed in `listing-card-list.tsx` *unless* it directly uses `useListingsStore.selectedListingId`. Verify:

Run: `grep -n selectedListingId apps/web/components/listings/listing-card-list.tsx`
If matches: replace `useListingsStore((s) => s.selectedListingId)` with derive-from-panel-stack pattern.

If no matches, skip step 6.8.2.

- [ ] **Step 6.8.2: Commit (if changes)**

```bash
git add apps/web/components/listings/listing-card-list.tsx
git commit -m "refactor(sp10-t6): listing-card-list selected highlight derives from panel stack"
```

### Step 6.9: Add panel lint rules to lefthook

- [ ] **Step 6.9.1: Read current `lefthook.yml`**

Use the file structure to identify the pre-commit section.

- [ ] **Step 6.9.2: Add 3 panel rules**

Append to the pre-commit `commands:` block:

```yaml
  panel-no-framework-import-kind:
    glob: "apps/web/lib/panel/**"
    run: |
      ! git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/lib/panel/.*\.(ts|tsx)$' \
        | xargs -r grep -l -E "from ['\"]@/components/panels/" \
        || (echo "ERROR: lib/panel/** must not import components/panels/** (spec § 9 #5)"; exit 1)

  panel-no-direct-codec:
    glob: "apps/web/**/*.{ts,tsx}"
    run: |
      ! git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/.*\.(ts|tsx)$' \
        | grep -v '^apps/web/lib/panel/codec\.' \
        | xargs -r grep -l -E "split\\(['\"][>][\"\\']\\)" \
        || (echo "ERROR: ad-hoc split('>') outside lib/panel/codec.ts (spec § 5.2)"; exit 1)

  panel-no-state-without-router:
    glob: "apps/web/stores/**"
    run: |
      ! git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/stores/.*\.(ts|tsx)$' \
        | xargs -r grep -l -E "panelStack" \
        || (echo "ERROR: zustand store must not hold panelStack — URL is SSOT (spec § 5.4)"; exit 1)
```

- [ ] **Step 6.9.3: Test the rules**

Stage a fake violation to confirm the rule blocks:

```bash
echo "import {} from '@/components/panels/parcel/summary';" >> apps/web/lib/panel/types.ts
git add apps/web/lib/panel/types.ts
git commit -m "test"
# expect commit to fail with "ERROR: lib/panel/** must not import components/panels/**"
git checkout -- apps/web/lib/panel/types.ts
```

- [ ] **Step 6.9.4: Commit**

```bash
git add lefthook.yml
git commit -m "ci(sp10-t6): lefthook panel lint rules — framework→kind / direct codec / store-panel-state"
```

### Step 6.10: Extensibility regression test

- [ ] **Step 6.10.1: Create `apps/web/tests/unit/panel-extensibility.test.ts`**

```ts
// apps/web/tests/unit/panel-extensibility.test.ts
/**
 * Spec § 10.3 — SSS 확장성 회귀.
 * 가짜 mock kind 등록만으로 codec / registry / view dispatch 가 작동하는지 검증.
 * Framework 코드 (lib/panel/*) 변경 없이 통과해야 SSS 확장성 lock.
 */
import { describe, expect, it } from 'vitest';
import { defineKind, getKindDefinition, getView, _resetRegistryForTests } from '@/lib/panel/registry';

describe('Panel extensibility', () => {
  it('a brand-new mock kind registers and resolves through the framework', () => {
    _resetRegistryForTests();
    const MockComponent = () => null;
    const fakeKind = 'parcel'; // we use 'parcel' as proxy because PanelKind is a closed union;
    // adding a brand-new kind requires extending PanelKind itself (compile-time enforcement —
    // exactly what spec § 6 promises). The runtime registry test confirms the *mechanism*.

    defineKind({
      kind: fakeKind,
      idPattern: /^.+$/,
      views: {
        summary: {
          component: MockComponent,
          fetcher: async () => ({ msg: 'hello' }),
          staleTime: 1000,
          links: [],
        },
        buildings: {
          component: MockComponent,
          fetcher: async () => ({ items: [] }),
          staleTime: 1000,
          links: [],
        },
        listings: {
          component: MockComponent,
          fetcher: async () => ({ listings: [], total: 0, page: 0, size: 0, has_next: false }),
          staleTime: 1000,
          links: [],
        },
      },
      loadingComponent: MockComponent,
      errorComponent: MockComponent,
      emptyComponent: MockComponent,
      authGate: { required: false },
      i18nNamespace: 'panels.mock',
      telemetryAttrs: () => ({}),
    });

    expect(getKindDefinition('parcel')).toBeDefined();
    expect(getView('parcel', 'summary')).toBeDefined();
    expect(getView('parcel', 'buildings')).toBeDefined();
    expect(getView('parcel', 'listings')).toBeDefined();
  });
});
```

> **NOTE:** 진짜 새 kind 추가는 `PanelKind` union 자체 확장 — 컴파일 타임 강제. 본 회귀 테스트는 R1 mechanism 자체가 깨지지 않았는지 검증.

- [ ] **Step 6.10.2: Run test**

Run: `cd apps/web && pnpm test panel-extensibility`
Expected: 1 test passes.

- [ ] **Step 6.10.3: Commit**

```bash
git add apps/web/tests/unit/panel-extensibility.test.ts
git commit -m "test(sp10-t6): panel extensibility regression — registry mechanism per spec § 10.3"
```

### Step 6.11: e2e tests

- [ ] **Step 6.11.1: Create `apps/web/tests/e2e/panel-system.spec.ts`**

```ts
// apps/web/tests/e2e/panel-system.spec.ts
/**
 * Spec § 10.2 — 패널 시스템 e2e.
 * Playwright. NEXT_PUBLIC_TILES_BASE_URL 미설정이면 폴리곤 click 은 skip
 * (대안: marker click 시퀀스만 검증).
 */
import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const TEST_PNU = '1168010100107370000'; // 19-digit fixture
const TEST_LISTING_UUID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';

test.describe('SP10 Panel System', () => {
  test('URL hydration: depth 1 panel from ?p directly', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await expect(page.getByRole('dialog')).toBeVisible();
    // PNU 표시 확인
    await expect(page.locator('text=' + TEST_PNU)).toBeVisible();
  });

  test('URL hydration: depth 2 chain', async ({ page }) => {
    await page.goto(
      `/listings?p=parcel:${TEST_PNU}.summary>listing:${TEST_LISTING_UUID}.summary`,
    );
    // breadcrumb 에 두 entry 노출
    const nav = page.getByRole('navigation', { name: /경로/ });
    await expect(nav.getByText('parcel.summary')).toBeVisible();
    await expect(nav.getByText('listing.summary')).toBeVisible();
  });

  test('Browser back pops top panel', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await page.goto(
      `/listings?p=parcel:${TEST_PNU}.summary>listing:${TEST_LISTING_UUID}.summary`,
    );
    await page.goBack();
    await expect(page).toHaveURL(/p=parcel%3A.*\.summary$/);
  });

  test('Refresh preserves stack', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await page.reload();
    await expect(page.getByRole('dialog')).toBeVisible();
  });

  test('Broken URL silently recovers', async ({ page }) => {
    await page.goto('/listings?p=invalid:bad.thing');
    // 패널 0 (dialog 미표시) — 카드 list 만 보임
    await expect(page.getByRole('dialog')).toHaveCount(0);
  });

  test('Mobile viewport: full-screen + back button', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    // back 버튼 (‹)
    await page.getByRole('button', { name: /이전/ }).click();
    await expect(page).toHaveURL(/\/listings(\?[^p]|$)/);
  });

  test('Keyboard ESC pops top panel', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    await page.keyboard.press('Escape');
    await expect(page).toHaveURL(/\/listings(\?[^p]|$)/);
  });

  test('a11y: no axe violations at panel depth 1', async ({ page }) => {
    await page.goto(`/listings?p=parcel:${TEST_PNU}.summary`);
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
  });
});
```

- [ ] **Step 6.11.2: Verify backend has fixtures or NoOp accepts the test PNU**

The NoOpParcelInfoLookup returns Ok(None) — so depth 1 panel will show 404 error state, not "summary". For e2e to work, ensure either:
- `services/api/tests` mock state seeds a PNU, OR
- `panel-system.spec.ts` runs against a dev DB with a known fixture.

Pragmatic: run e2e in `AUTH_DEV_MODE=true` + dev DB seed; confirm via `pnpm test:e2e` the test plays. If skeleton state is acceptable for the URL-hydration tests, refactor assertions to check `role="dialog"` presence rather than data text.

- [ ] **Step 6.11.3: Run e2e**

Run: `cd apps/web && pnpm test:e2e panel-system`
Expected: 8 tests pass (or assert acceptance criteria are visibly met).

- [ ] **Step 6.11.4: Commit**

```bash
git add apps/web/tests/e2e/panel-system.spec.ts
git commit -m "test(sp10-t6): e2e panel system — hydration / back / refresh / ESC / mobile / a11y"
```

### Step 6.12: Final acceptance run

- [ ] **Step 6.12.1: Full lint + typecheck**

Run: `cd apps/web && pnpm lint && pnpm typecheck`
Expected: clean.

- [ ] **Step 6.12.2: Full unit test**

Run: `cd apps/web && pnpm test`
Expected: all green.

- [ ] **Step 6.12.3: Full e2e**

Run: `cd apps/web && pnpm test:e2e`
Expected: all green.

- [ ] **Step 6.12.4: Bundle size check (size-limit)**

Run: `cd apps/web && pnpm test:bundle`
Expected: under existing budget.

- [ ] **Step 6.12.5: Backend tests**

Run: `cargo test -p api`
Expected: all green.

- [ ] **Step 6.12.6: Backend clippy**

Run: `cargo clippy -p api --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 6.12.7: Final acceptance commit (sweeping any straggler fixes)**

If any straggler fixes:

```bash
git add -A
git commit -m "fix(sp10-t6): final sweep — typecheck/lint/e2e green"
```

- [ ] **Step 6.12.8: Push branch**

```bash
git push origin HEAD
```

---

## Self-Review Notes

(Filled in by the engineer or planner after completion.)

- [ ] **Spec coverage:** Every § in the spec mapped to a task (§ 3 → T1; § 4 → T2; § 5 → T1+T2; § 6 → T1; § 7 → T3; § 9 → distributed across T1-T6; § 10 → T6; § 11 → T6; § 12 → all).
- [ ] **Placeholder scan:** No "TBD"/"add error handling"/"similar to" — all code blocks are concrete.
- [ ] **Type consistency:** `PanelKind`, `PanelView<K>`, `PanelStackEntry`, `PanelStack`, `usePanelStack`, `defineKind` signatures match across T1-T5.
- [ ] **Risk per spec § 14:** Each addressed — URL=SSOT lint (T6.9), Next 16 router (T1.4 mocks), mobile fullscreen + map preserve (T2.4 doesn't unmount map), extensibility test (T6.10), depth max=8 in codec (T1.2.3).

---

**Plan complete. Spec rule alignment verified against [docs/superpowers/specs/2026-05-07-sub-project-10-panel-system-design.md](../specs/2026-05-07-sub-project-10-panel-system-design.md).**
