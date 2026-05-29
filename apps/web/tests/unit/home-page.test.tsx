import { describe, expect, it, vi } from "vitest";

const redirectMock = vi.hoisted(() =>
  vi.fn((href: string) => {
    throw new Error(`NEXT_REDIRECT:${href}`);
  }),
);

vi.mock("next/navigation", () => ({
  redirect: redirectMock,
}));

describe("home page", () => {
  it("redirects to the listings app entry", async () => {
    const { default: Home } = await import("@/app/page");

    expect(() => Home()).toThrow("NEXT_REDIRECT:/listings");
    expect(redirectMock).toHaveBeenCalledWith("/listings");
  });
});
