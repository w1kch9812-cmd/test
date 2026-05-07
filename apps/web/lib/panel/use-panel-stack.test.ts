// apps/web/lib/panel/use-panel-stack.test.ts
import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mockPush = vi.fn();
const mockBack = vi.fn();
const mockSearchParams = new URLSearchParams();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: mockPush, back: mockBack, replace: vi.fn() }),
  useSearchParams: () => mockSearchParams,
  usePathname: () => "/listings",
}));

import { usePanelStack } from "./use-panel-stack";

beforeEach(() => {
  mockPush.mockClear();
  mockBack.mockClear();
  mockSearchParams.delete("p");
});

describe("usePanelStack", () => {
  it("returns empty stack when ?p missing", () => {
    const { result } = renderHook(() => usePanelStack());
    expect(result.current.stack.entries).toHaveLength(0);
  });

  it("hydrates stack from ?p search param", () => {
    mockSearchParams.set("p", "parcel:1168010100107370000.summary");
    const { result } = renderHook(() => usePanelStack());
    expect(result.current.stack.entries).toHaveLength(1);
    expect(result.current.stack.entries[0]).toEqual({
      kind: "parcel",
      id: "1168010100107370000",
      view: "summary",
    });
  });

  it("push calls router.push with serialized url", () => {
    const { result } = renderHook(() => usePanelStack());
    act(() => {
      result.current.push({ kind: "parcel", id: "1168010100107370000", view: "summary" });
    });
    expect(mockPush).toHaveBeenCalledWith("/listings?p=parcel%3A1168010100107370000.summary", {
      scroll: false,
    });
  });

  it("pop calls router.back", () => {
    mockSearchParams.set(
      "p",
      "parcel:1168010100107370000.summary>listing:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.summary",
    );
    const { result } = renderHook(() => usePanelStack());
    act(() => {
      result.current.pop();
    });
    expect(mockBack).toHaveBeenCalledTimes(1);
  });

  it("silent recover from broken url (empty stack)", () => {
    mockSearchParams.set("p", "invalid:bad.thing");
    const { result } = renderHook(() => usePanelStack());
    expect(result.current.stack.entries).toHaveLength(0);
    // depth-0 = silent recover (Sentry 이벤트는 telemetry.test.ts 가 검증)
  });
});
