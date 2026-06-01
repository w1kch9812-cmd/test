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

