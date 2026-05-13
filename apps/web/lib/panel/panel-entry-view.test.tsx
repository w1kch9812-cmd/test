// apps/web/lib/panel/panel-entry-view.test.tsx
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { NextIntlClientProvider } from "next-intl";
import type React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PanelEntryView } from "./panel-entry-view";
import { _resetRegistryForTests, defineKind } from "./registry";

// Mock telemetry to avoid OTEL noise
vi.mock("./telemetry", () => ({
  reportPanelOpened: vi.fn(),
  reportUrlDecodeFailed: vi.fn(),
}));

// Mock usePanelStack to avoid Next.js navigation
vi.mock("./use-panel-stack", () => ({
  usePanelStack: () => ({
    stack: { v: 1, entries: [] },
    push: () => {},
    pop: () => {},
    truncate: () => {},
  }),
}));

afterEach(() => {
  _resetRegistryForTests();
  vi.clearAllMocks();
});

const ThrowingComponent = () => {
  throw new Error("registry component blew up");
};
const ErrorCard = ({ error }: { error: unknown }) => (
  <div>ERROR: {error instanceof Error ? error.message : String(error)}</div>
);
const Loading = () => <div>L</div>;
const Empty = () => <div>E</div>;
const messages = {
  panel: {
    labels: {
      parcel: {
        summary: "Parcel summary",
        buildings: "Parcel buildings",
        listings: "Parcel listings",
      },
      listing: {
        summary: "Listing summary",
      },
    },
  },
};

function makeRegistry() {
  defineKind({
    kind: "parcel",
    idPattern: /^\d{19}$/,
    views: {
      summary: {
        component: ThrowingComponent,
        fetcher: async () => ({ ok: true }),
        staleTime: 1000,
        links: [],
      },
      buildings: {
        component: () => null,
        fetcher: async () => ({ items: [] }),
        staleTime: 1000,
        links: [],
      },
      listings: {
        component: () => null,
        fetcher: async () => ({
          listings: [],
          total: 0,
          page: 0,
          size: 0,
          has_next: false,
        }),
        staleTime: 1000,
        links: [],
      },
    },
    loadingComponent: Loading,
    errorComponent: ErrorCard,
    emptyComponent: Empty,
    authGate: { required: false },
    i18nNamespace: "panels.parcel",
    telemetryAttrs: () => ({}),
  });
}

function renderWithQuery(ui: React.ReactNode) {
  // disable retries so the test does not loop
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <NextIntlClientProvider locale="ko" messages={messages}>
      <QueryClientProvider client={qc}>{ui}</QueryClientProvider>
    </NextIntlClientProvider>,
  );
}

describe("PanelEntryView ErrorBoundary", () => {
  // Suppress React's expected error logging for this test
  beforeEach(() => {
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("catches render-time exception from registry component → state=error", async () => {
    makeRegistry();
    renderWithQuery(
      <PanelEntryView
        entry={{ kind: "parcel", id: "1168010100107370000", view: "summary" }}
        depth={1}
      />,
    );
    await waitFor(() => {
      expect(screen.getByText(/registry component blew up/)).toBeInTheDocument();
    });
  });
});
