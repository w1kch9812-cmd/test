import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const ZITADEL_REAL = process.env.ZITADEL_E2E_REAL === "true";

test.describe("listings search", () => {
  test("/(authenticated)/listings 미인증 → /login redirect", async ({ page }) => {
    await page.goto("/listings");
    await page.waitForURL(/\/login/, { timeout: 10000 });
    expect(page.url()).toContain("returnTo=%2Flistings");
  });

  test("a11y on /listings (real Zitadel — CI 에서는 skip)", async ({ page }) => {
    test.skip(!ZITADEL_REAL, "real Zitadel container required");

    // 로그인 흐름 (auth.spec.ts 패턴 일관)
    await page.goto("/login");
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/ui\/login\/login/, { timeout: 15000 });
    await page.fill('input[name="loginName"]', "admin@zitadel.localhost");
    await page.click('button[type="submit"]');
    await page.fill('input[name="password"]', "Admin123!");
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/profile|\/listings/, { timeout: 30000 });
    await page.goto("/listings");
    await page.waitForLoadState("networkidle");

    const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();

    const criticalViolations = results.violations.filter(
      (v) => v.impact === "critical" || v.impact === "serious",
    );

    if (criticalViolations.length > 0) {
      console.error(
        `[a11y] ${criticalViolations.length} critical/serious violations:`,
        JSON.stringify(criticalViolations, null, 2),
      );
    }

    expect(criticalViolations).toEqual([]);
  });

  test("필터 chip 클릭 → aria-pressed 동기화 (real Zitadel — CI 에서는 skip)", async ({ page }) => {
    test.skip(!ZITADEL_REAL, "real Zitadel container required");

    // 로그인 흐름 (auth.spec.ts 패턴 일관)
    await page.goto("/login");
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/ui\/login\/login/, { timeout: 15000 });
    await page.fill('input[name="loginName"]', "admin@zitadel.localhost");
    await page.click('button[type="submit"]');
    await page.fill('input[name="password"]', "Admin123!");
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/profile|\/listings/, { timeout: 30000 });
    await page.goto("/listings");
    await page.waitForLoadState("networkidle");

    // 종류 chip click — MultiSelect button[aria-pressed]
    const factoryChip = page.getByRole("button", { name: "공장", exact: true });
    await expect(factoryChip).toHaveAttribute("aria-pressed", "false");
    await factoryChip.click();
    await expect(factoryChip).toHaveAttribute("aria-pressed", "true");
  });
});
