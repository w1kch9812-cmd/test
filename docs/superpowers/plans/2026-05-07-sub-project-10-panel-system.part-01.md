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

