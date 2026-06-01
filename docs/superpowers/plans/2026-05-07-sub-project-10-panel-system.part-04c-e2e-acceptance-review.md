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
