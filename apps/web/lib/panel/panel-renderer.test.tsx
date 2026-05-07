// apps/web/lib/panel/panel-renderer.test.tsx
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

interface MatchMediaMock {
  (q: string): void;
  matchesValue: boolean;
}
const matchMediaMock = vi.fn() as unknown as MatchMediaMock;
beforeEach(() => {
  (matchMediaMock as unknown as ReturnType<typeof vi.fn>).mockReset();
});

vi.stubGlobal("matchMedia", (q: string) => {
  matchMediaMock(q);
  return {
    matches: q.includes("1280") ? matchMediaMock.matchesValue : false,
    addEventListener: () => {},
    removeEventListener: () => {},
  };
});

vi.mock("./side-by-side-stack", () => ({
  SideBySideStack: () => <div>SIDE_BY_SIDE</div>,
}));
vi.mock("./full-screen-stack", () => ({
  FullScreenStack: () => <div>FULL_SCREEN</div>,
}));
vi.mock("./use-panel-stack", () => ({
  usePanelStack: () => ({
    stack: { v: 1, entries: [{ kind: "parcel", id: "1168010100107370000", view: "summary" }] },
    push: () => {},
    pop: () => {},
    truncate: () => {},
  }),
}));

import { PanelRenderer } from "./panel-renderer";

describe("PanelRenderer", () => {
  it("renders SideBySideStack at >= xl viewport", () => {
    matchMediaMock.matchesValue = true;
    render(<PanelRenderer />);
    expect(screen.getByText("SIDE_BY_SIDE")).toBeInTheDocument();
  });

  it("renders FullScreenStack at < xl viewport", () => {
    matchMediaMock.matchesValue = false;
    render(<PanelRenderer />);
    expect(screen.getByText("FULL_SCREEN")).toBeInTheDocument();
  });
});
