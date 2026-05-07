// apps/web/lib/panel/focus-trap.test.ts
import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useFocusTrap } from "./focus-trap";

describe("useFocusTrap", () => {
  it("moves focus to container on mount, restores on unmount", () => {
    const prev = document.createElement("button");
    document.body.appendChild(prev);
    prev.focus();
    expect(document.activeElement).toBe(prev);

    const container = document.createElement("div");
    container.tabIndex = -1;
    document.body.appendChild(container);

    const { unmount } = renderHook(() => useFocusTrap({ current: container }));
    expect(document.activeElement).toBe(container);

    unmount();
    expect(document.activeElement).toBe(prev);

    document.body.removeChild(prev);
    document.body.removeChild(container);
  });
});
