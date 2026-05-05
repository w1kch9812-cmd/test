import { z } from "zod";
import { api } from "@/lib/api";
import type { ListingFilters } from "@/lib/listings/filters";
import { toSearchParams } from "@/lib/listings/filters";

export const ListingCardSchema = z.object({
  id: z.string(),
  title: z.string(),
  listing_type: z.enum([
    "factory",
    "warehouse",
    "office",
    "knowledge_industry_center",
    "industrial_land",
    "logistics_center",
  ]),
  transaction_type: z.enum(["sale", "monthly_rent", "jeonse"]),
  price_krw: z.number().int(),
  deposit_krw: z.number().int().nullable(),
  monthly_rent_krw: z.number().int().nullable(),
  area_m2: z.number(),
  lat: z.number(),
  lng: z.number(),
  thumbnail_url: z.string().nullable(),
  view_count: z.number().int(),
  bookmark_count: z.number().int(),
  created_at: z.string(), // ISO 8601
});

export type ListingCard = z.infer<typeof ListingCardSchema>;

export const ListingsResponseSchema = z.object({
  listings: z.array(ListingCardSchema),
  total: z.number().int(),
  page: z.number().int(),
  size: z.number().int(),
  has_next: z.boolean(),
});

export type ListingsResponse = z.infer<typeof ListingsResponseSchema>;

export interface FetchListingsInput {
  filters: ListingFilters;
  bounds?: { south: number; west: number; north: number; east: number };
  page?: number;
  size?: number;
}

export async function fetchListings(input: FetchListingsInput): Promise<ListingsResponse> {
  const sp = toSearchParams(input.filters);
  if (input.bounds) {
    const { south, west, north, east } = input.bounds;
    sp.set("bounds", `${south},${west},${north},${east}`);
  }
  if (input.page !== undefined) sp.set("page", String(input.page));
  if (input.size !== undefined) sp.set("size", String(input.size));

  const json = await api.get(`listings?${sp.toString()}`).json<unknown>();
  return ListingsResponseSchema.parse(json);
}
