// apps/web/lib/panel/panel-card.test.tsx
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PanelCard } from "./panel-card";

describe("PanelCard", () => {
  it("renders loadingComponent when isLoading", () => {
    render(
      <PanelCard
        state="loading"
        onClose={() => {}}
        loading={<div>LOADING</div>}
        error={<div>ERR</div>}
        empty={<div>EMPTY</div>}
        authRequired={<div>AUTH</div>}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    expect(screen.getByText("LOADING")).toBeInTheDocument();
  });

  it("renders errorComponent when state=error", () => {
    render(
      <PanelCard
        state="error"
        onClose={() => {}}
        loading={<div>LOADING</div>}
        error={<div>ERR</div>}
        empty={<div>EMPTY</div>}
        authRequired={<div>AUTH</div>}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    expect(screen.getByText("ERR")).toBeInTheDocument();
  });

  it("renders content when state=ok", () => {
    render(
      <PanelCard
        state="ok"
        onClose={() => {}}
        loading={<div>LOADING</div>}
        error={<div>ERR</div>}
        empty={<div>EMPTY</div>}
        authRequired={<div>AUTH</div>}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    expect(screen.getByText("CONTENT")).toBeInTheDocument();
  });

  it("calls onClose on ESC keydown", () => {
    const onClose = vi.fn();
    render(
      <PanelCard
        state="ok"
        onClose={onClose}
        loading={null}
        error={null}
        empty={null}
        authRequired={null}
      >
        <div>CONTENT</div>
      </PanelCard>,
    );
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("has aria-modal=true and role=dialog", () => {
    render(
      <PanelCard
        state="ok"
        onClose={() => {}}
        loading={null}
        error={null}
        empty={null}
        authRequired={null}
      >
        <div />
      </PanelCard>,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
  });
});
