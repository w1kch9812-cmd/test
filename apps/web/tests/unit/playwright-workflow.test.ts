import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const FRONTEND_WORKFLOW = resolve(__dirname, "../../../../.github/workflows/frontend.yml");

describe("frontend workflow Playwright runtime", () => {
  it("does not override the Playwright-owned local callback URL", () => {
    const workflow = readFileSync(FRONTEND_WORKFLOW, "utf8");

    expect(workflow).not.toContain("ZITADEL_REDIRECT_URI: http://localhost:3000/api/auth/callback");
    expect(workflow).toContain("Playwright runtime derives the local callback URL");
  });
});
