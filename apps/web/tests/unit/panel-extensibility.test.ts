// apps/web/tests/unit/panel-extensibility.test.ts
/**
 * Spec § 10.3 — SSS 확장성 회귀.
 * 가짜 mock kind 등록만으로 codec / registry / view dispatch 가 작동하는지 검증.
 * Framework 코드 (lib/panel/*) 변경 없이 통과해야 SSS 확장성 lock.
 *
 * NOTE: 진짜 새 kind 추가는 `PanelKind` union 자체 확장 — 컴파일 타임 강제.
 * 본 회귀 테스트는 R1 mechanism 자체가 깨지지 않았는지 검증.
 */
import { describe, expect, it } from "vitest";
import {
  _resetRegistryForTests,
  defineKind,
  getKindDefinition,
  getView,
} from "@/lib/panel/registry";

describe("Panel extensibility", () => {
  it("a brand-new mock kind registers and resolves through the framework", () => {
    _resetRegistryForTests();
    const MockComponent = () => null;
    // We use 'parcel' as proxy because PanelKind is a closed union;
    // adding a brand-new kind requires extending PanelKind itself (compile-time enforcement —
    // exactly what spec § 6 promises). The runtime registry test confirms the *mechanism*.
    const fakeKind = "parcel" as const;

    defineKind({
      kind: fakeKind,
      idPattern: /^.+$/,
      views: {
        summary: {
          component: MockComponent,
          fetcher: async () => ({ msg: "hello" }),
          staleTime: 1000,
          links: [],
        },
        buildings: {
          component: MockComponent,
          fetcher: async () => ({ items: [] }),
          staleTime: 1000,
          links: [],
        },
        listings: {
          component: MockComponent,
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
      loadingComponent: MockComponent,
      errorComponent: MockComponent,
      emptyComponent: MockComponent,
      authGate: { required: false },
      i18nNamespace: "panels.mock",
      telemetryAttrs: () => ({}),
    });

    expect(getKindDefinition("parcel")).toBeDefined();
    expect(getView("parcel", "summary")).toBeDefined();
    expect(getView("parcel", "buildings")).toBeDefined();
    expect(getView("parcel", "listings")).toBeDefined();
  });
});
