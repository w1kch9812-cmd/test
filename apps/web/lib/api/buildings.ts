// apps/web/lib/api/buildings.ts
import { z } from "zod";
import { api } from "@/lib/api";

export const BuildingSchema = z.object({
  id: z.string(),
  name: z.string(),
  purpose: z.string(),
  total_area_m2: z.number(),
  approved_at: z.string().nullish(),
});

export type Building = z.infer<typeof BuildingSchema>;

export const BuildingsResponseSchema = z.object({
  buildings: z.array(BuildingSchema),
});

export type BuildingsResponse = z.infer<typeof BuildingsResponseSchema>;

export async function fetchBuildings(
  parcelPnu: string,
  signal?: AbortSignal,
): Promise<BuildingsResponse> {
  const json = await api
    .get(`api/buildings?parcel_pnu=${encodeURIComponent(parcelPnu)}`, { signal })
    .json<unknown>();
  return BuildingsResponseSchema.parse(json);
}
