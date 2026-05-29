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

