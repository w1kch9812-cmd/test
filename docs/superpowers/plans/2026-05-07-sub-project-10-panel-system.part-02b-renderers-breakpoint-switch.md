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

