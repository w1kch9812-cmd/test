# SP10 Panel System - Part 01C: Telemetry and Focus Trap

Parent index: [SP10 Panel System - Part 01](./2026-05-07-sub-project-10-panel-system.part-01.md).
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
