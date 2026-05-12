/**
 * Listing pin color resolver. 색상 SSOT 는 @gongzzang/ui/tokens.js.
 */
import {
  LISTING_TYPE_COLOR_FALLBACK,
  LISTING_TYPE_COLORS,
  type ListingTypeKey,
} from "@gongzzang/ui/tokens.js";

// re-export for backwards compat
export { LISTING_TYPE_COLORS, type ListingTypeKey };

export function getPinColor(listingType: string): string {
  return LISTING_TYPE_COLORS[listingType as ListingTypeKey] ?? LISTING_TYPE_COLOR_FALLBACK;
}
