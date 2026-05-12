import { PIN_COLORS } from "@gongzzang/ui/tokens.js";
import { getPinColor } from "@/lib/listings/pin-color";

/**
 * Naver Maps 의 marker icon 으로 사용할 SVG HTML string.
 * `new naver.maps.Marker({ icon: { content: pinIconHtml(...) } })`.
 * 색상 SSOT — @gongzzang/ui/tokens.js 의 PIN_COLORS / LISTING_TYPE_COLORS.
 */
export function pinIconHtml(listingType: string, options: { selected?: boolean } = {}): string {
  const color = getPinColor(listingType);
  const size = options.selected ? 36 : 28;
  const stroke = options.selected ? PIN_COLORS.strokeSelected : PIN_COLORS.strokeIdle;
  const strokeWidth = options.selected ? 3 : 1.5;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="${color}" stroke="${stroke}" stroke-width="${strokeWidth}">
    <path d="M12 2C7.58 2 4 5.58 4 10c0 5.25 7 12 8 12s8-6.75 8-12c0-4.42-3.58-8-8-8z"/>
    <circle cx="12" cy="10" r="3" fill="${PIN_COLORS.inner}"/>
  </svg>`;
}
