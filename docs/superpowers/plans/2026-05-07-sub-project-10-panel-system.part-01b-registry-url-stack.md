# SP10 Panel System - Part 01B: Registry and URL Stack

Parent index: [SP10 Panel System - Part 01](./2026-05-07-sub-project-10-panel-system.part-01.md).
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
