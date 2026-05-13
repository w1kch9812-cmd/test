import { describe, expect, it } from "vitest";
import "./panel-renderer";
import { getView } from "./registry";

describe("PanelRenderer registry bootstrap", () => {
  it("registers default panel views in the client module graph", () => {
    expect(getView("parcel", "summary")).toBeDefined();
    expect(getView("parcel", "buildings")).toBeDefined();
    expect(getView("parcel", "listings")).toBeDefined();
    expect(getView("listing", "summary")).toBeDefined();
  });
});
