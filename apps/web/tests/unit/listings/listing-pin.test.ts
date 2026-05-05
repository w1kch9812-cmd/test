// @vitest-environment node
import { describe, expect, it } from "vitest";
import { pinIconHtml } from "@/components/listings/listing-pin";

describe("pinIconHtml", () => {
  it("default size 28, stroke #1f2937", () => {
    const html = pinIconHtml("factory");
    expect(html).toContain('width="28"');
    expect(html).toContain('stroke="#1f2937"');
    expect(html).toContain('fill="#dc2626"'); // factory red
  });

  it("selected size 36, stroke white", () => {
    const html = pinIconHtml("warehouse", { selected: true });
    expect(html).toContain('width="36"');
    expect(html).toContain('stroke="#ffffff"');
    expect(html).toContain('fill="#2563eb"'); // warehouse blue
  });

  it("unknown listing_type fallback gray", () => {
    const html = pinIconHtml("unknown");
    expect(html).toContain('fill="#6b7280"'); // gray fallback
  });

  it("SVG root element present", () => {
    const html = pinIconHtml("office");
    expect(html).toContain("<svg ");
    expect(html).toContain("</svg>");
  });

  it("selected stroke-width 3, default stroke-width 1.5", () => {
    const selected = pinIconHtml("factory", { selected: true });
    const normal = pinIconHtml("factory");
    expect(selected).toContain('stroke-width="3"');
    expect(normal).toContain('stroke-width="1.5"');
  });
});
