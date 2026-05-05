export const LISTING_TYPE_COLORS = {
  factory: "#dc2626", // red-600 (공장)
  warehouse: "#2563eb", // blue-600 (창고)
  office: "#059669", // emerald-600 (사무실)
  knowledge_industry_center: "#7c3aed", // violet-600 (지식산업센터)
  industrial_land: "#ea580c", // orange-600 (산업단지/토지)
  logistics_center: "#0891b2", // cyan-600 (물류센터)
} as const;

export type ListingTypeKey = keyof typeof LISTING_TYPE_COLORS;

export function getPinColor(listingType: string): string {
  return LISTING_TYPE_COLORS[listingType as ListingTypeKey] ?? "#6b7280"; // gray-500 fallback
}
