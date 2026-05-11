// apps/web/lib/api/parcels.ts
import { z } from "zod";
import { api } from "@/lib/api";

export const ParcelInfoSchema = z.object({
  pnu: z.string(),
  sido_code: z.string(),
  sigungu_code: z.string(),
  eupmyeondong_code: z.string(),
  sido_name: z.string(),
  sigungu_name: z.string(),
  eupmyeondong_name: z.string(),
  land_use_type: z.string(),
  zoning: z.string().nullish(),
  official_land_price_per_m2: z.number().int().nullish(),
  gosi_year_month: z.string().nullish(),
});

export type ParcelInfo = z.infer<typeof ParcelInfoSchema>;

export async function fetchParcel(pnu: string, signal?: AbortSignal): Promise<ParcelInfo> {
  const json = await api.get(`api/parcels/${pnu}`, { signal }).json<unknown>();
  return ParcelInfoSchema.parse(json);
}
