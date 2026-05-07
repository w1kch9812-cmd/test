// apps/web/lib/panel/registry.test.ts
import { afterEach, describe, expect, it } from "vitest";
import { _resetRegistryForTests, defineKind, getKindDefinition, getView } from "./registry";

afterEach(() => {
  _resetRegistryForTests();
});

const DummyComponent = () => null;

describe("registry", () => {
  it("registers and retrieves a kind", () => {
    defineKind({
      kind: "parcel",
      idPattern: /^\d{19}$/,
      views: {
        summary: {
          component: DummyComponent,
          fetcher: async () => ({}),
          staleTime: 60_000,
          links: [],
        },
      },
      loadingComponent: DummyComponent,
      errorComponent: DummyComponent,
      emptyComponent: DummyComponent,
      authGate: { required: false },
      i18nNamespace: "panels.parcel",
      telemetryAttrs: () => ({}),
    });
    const def = getKindDefinition("parcel");
    expect(def?.kind).toBe("parcel");
  });

  it("throws on duplicate registration", () => {
    const def = {
      kind: "parcel" as const,
      idPattern: /^\d{19}$/,
      views: {
        summary: {
          component: DummyComponent,
          fetcher: async () => ({}),
          staleTime: 60_000,
          links: [],
        },
      },
      loadingComponent: DummyComponent,
      errorComponent: DummyComponent,
      emptyComponent: DummyComponent,
      authGate: { required: false } as const,
      i18nNamespace: "panels.parcel",
      telemetryAttrs: () => ({}),
    };
    defineKind(def);
    expect(() => defineKind(def)).toThrowError(/already registered/i);
  });

  it("returns undefined for unregistered kind", () => {
    expect(getKindDefinition("parcel")).toBeUndefined();
  });

  it("getView returns view config for registered kind+view", () => {
    defineKind({
      kind: "parcel",
      idPattern: /^\d{19}$/,
      views: {
        summary: {
          component: DummyComponent,
          fetcher: async () => ({}),
          staleTime: 60_000,
          links: [],
        },
      },
      loadingComponent: DummyComponent,
      errorComponent: DummyComponent,
      emptyComponent: DummyComponent,
      authGate: { required: false },
      i18nNamespace: "panels.parcel",
      telemetryAttrs: () => ({}),
    });
    expect(getView("parcel", "summary")).toBeDefined();
  });
});
