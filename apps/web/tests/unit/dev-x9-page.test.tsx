import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const notFoundMock = vi.hoisted(() =>
  vi.fn(() => {
    throw new Error("NEXT_NOT_FOUND");
  }),
);

vi.mock("next/navigation", () => ({
  notFound: notFoundMock,
}));

describe("dev-x9-test page", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllEnvs();
    notFoundMock.mockClear();
  });

  it("is not exposed in production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    const { default: DevX9TestPage } = await import("@/app/dev-x9-test/page");

    expect(() => render(<DevX9TestPage />)).toThrow("NEXT_NOT_FOUND");
    expect(notFoundMock).toHaveBeenCalled();
  });
});
