import { describe, expect, it } from "vitest";
import { env } from "@/lib/env";

describe("env validation", () => {
  it("provides default API_BASE_URL", () => {
    expect(env.NEXT_PUBLIC_API_BASE_URL).toBeDefined();
    expect(env.NEXT_PUBLIC_API_BASE_URL).toMatch(/^https?:\/\//);
  });
});
