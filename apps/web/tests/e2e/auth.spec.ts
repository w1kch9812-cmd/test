import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.describe("auth flow", () => {
  test("/login is publicly accessible + a11y", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByRole("heading", { name: "로그인" })).toBeVisible();
    await expect(page.getByRole("button", { name: "로그인하기" })).toBeVisible();

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
  });

  test("unauthenticated /profile redirects to /login", async ({ page }) => {
    await page.goto("/profile");
    await page.waitForURL(/\/login/);
    expect(page.url()).toContain("returnTo=%2Fprofile");
  });

  test("/forbidden displays role-mismatch message + a11y", async ({ page }) => {
    await page.goto("/forbidden");
    await expect(page.getByRole("heading", { name: "접근 권한이 없어요" })).toBeVisible();

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
  });

  // 실 Zitadel container 의존 — 사용자 인증 흐름
  test("login → callback (Zitadel hosted UI) → profile", async ({ page }) => {
    test.skip(
      process.env.ZITADEL_E2E_REAL !== "true",
      "real Zitadel container required (set ZITADEL_E2E_REAL=true)",
    );

    await page.goto("/login");
    await page.click('button[type="submit"]');
    // Zitadel hosted UI
    await page.waitForURL(/\/ui\/login\/login/);
    await page.fill('input[name="loginName"]', "admin@zitadel.localhost");
    await page.click('button[type="submit"]');
    await page.fill('input[name="password"]', "Admin123!");
    await page.click('button[type="submit"]');
    // 로그인 후 /profile 도달
    await page.waitForURL(/\/profile$/, { timeout: 60000 });
    await expect(page.getByRole("heading", { name: "내 정보" })).toBeVisible();
  });

  test("logout returns to root with cookie cleared", async ({ page, context }) => {
    test.skip(
      process.env.ZITADEL_E2E_REAL !== "true",
      "real Zitadel container required (set ZITADEL_E2E_REAL=true)",
    );

    await page.goto("/profile");
    await page.click('button:has-text("로그아웃")');
    await page.waitForURL("/", { timeout: 30000 });
    const cookies = await context.cookies();
    expect(cookies.find((c) => c.name === "__Host-sid")).toBeUndefined();
  });
});
