import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.describe("a11y — WCAG 2.1 AA", () => {
  test("home page passes axe (critical/serious 0)", async ({ page }) => {
    await page.goto("/");
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

  test("not-found page passes axe", async ({ page }) => {
    await page.goto("/__nonexistent-path-for-testing__");
    await page.waitForLoadState("networkidle");

    const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();

    const criticalViolations = results.violations.filter(
      (v) => v.impact === "critical" || v.impact === "serious",
    );

    expect(criticalViolations).toEqual([]);
  });
});
