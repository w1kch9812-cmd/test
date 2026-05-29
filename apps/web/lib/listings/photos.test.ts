import { describe, expect, it } from "vitest";
import { listingPhotoImageSrc } from "./photos";

describe("listing photo urls", () => {
  it("uses the authenticated proxy path with photo_id instead of r2_key", () => {
    const src = listingPhotoImageSrc("lst_123", {
      photo_id: "lph_456",
      r2_key: "listings/lst_123/lph_456.jpg",
    });

    expect(src).toBe("/api/proxy/listings/lst_123/photos/lph_456");
    expect(src).not.toContain("listings/lst_123/lph_456.jpg");
  });
});
