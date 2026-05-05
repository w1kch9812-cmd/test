import { expect, test } from "@playwright/test";

test.describe("Foundation smoke", () => {
  test("home page loads + healthz call (mocked or real backend)", async ({ page }) => {
    // Backend 안 떠 있으면 502 — 의도된 fail 처리.
    // smoke 의 의미 = "frontend pipeline 정상" 확인 (200 또는 502 둘 다 OK).

    await page.goto("/");

    await expect(page.getByText("공짱 Foundation Smoke")).toBeVisible();

    await page.waitForFunction(
      () => {
        const el = document.querySelector("[data-testid='healthz-response']");
        const errEl = document.querySelector("[role='alert']");
        return el !== null || errEl !== null;
      },
      { timeout: 10000 },
    );

    const responseEl = page.getByTestId("healthz-response");
    const errorEl = page.getByRole("alert");

    const hasResponse = (await responseEl.count()) > 0;
    const hasError = (await errorEl.count()) > 0;

    expect(hasResponse || hasError).toBe(true);
  });
});
