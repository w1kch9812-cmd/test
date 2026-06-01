// apps/web/lib/api/buildings.ts
import { z } from "zod";
import { apiProxyClient } from "@/lib/api/api-proxy-client.generated";

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
  const searchParams = new URLSearchParams({ parcel_pnu: parcelPnu });
  const json = await apiProxyClient.buildingsRead.getJson<unknown>({ searchParams, signal });
  return BuildingsResponseSchema.parse(json);
}
