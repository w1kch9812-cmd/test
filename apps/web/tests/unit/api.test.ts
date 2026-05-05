import { describe, expect, it } from "vitest";
import { api } from "@/lib/api";

describe("api client", () => {
  it("is defined and callable", () => {
    expect(api).toBeDefined();
    expect(typeof api.get).toBe("function");
    expect(typeof api.post).toBe("function");
  });
});
