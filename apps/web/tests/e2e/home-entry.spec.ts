import { expect, test } from "@playwright/test";

test.describe("home entry", () => {
  test("redirects unauthenticated visitors to the login-gated listings entry", async ({ page }) => {
    await page.goto("/");

    await page.waitForURL(/\/login/, { timeout: 10000 });
    expect(page.url()).toContain("returnTo=%2Flistings");
  });
});
