// apps/web/components/panels/parcel/register.ts
// Side-effect-only module: importing this file triggers defineKind('parcel') once.
// No exports. T6 imports it from app/listings/page.tsx for registration.

import { type BuildingsResponse, fetchBuildings } from "@/lib/api/buildings";
import { fetchParcel, type ParcelInfo } from "@/lib/api/parcels";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { defineKind, defineView } from "@/lib/panel/registry";
import { ParcelBuildingsCard } from "./buildings";
import { ParcelListingsCard } from "./listings";
import { ParcelEmptyCard, ParcelErrorCard, ParcelLoadingSkeleton } from "./skeletons";
import { ParcelSummaryCard } from "./summary";

defineKind({
  kind: "parcel",
  idPattern: /^\d{19}$/,
  views: {
    summary: defineView<"parcel", ParcelInfo>({
      component: ParcelSummaryCard,
      fetcher: (id) => fetchParcel(id),
      staleTime: 5 * 60_000,
      links: [],
    }),
    buildings: defineView<"parcel", BuildingsResponse>({
      component: ParcelBuildingsCard,
      fetcher: (id) => fetchBuildings(id),
      staleTime: 5 * 60_000,
      links: [],
    }),
    listings: defineView<"parcel", ListingsResponse>({
      component: ParcelListingsCard,
      fetcher: (id) =>
        fetchListings({
          filters: {
            types: [],
            transactions: [],
            minAreaM2: undefined,
            maxAreaM2: undefined,
            minPriceKrw: undefined,
            maxPriceKrw: undefined,
            sort: "created_at_desc",
            adminCode: undefined,
            landUseType: undefined,
          },
          pnu: id,
        }),
      staleTime: 60_000,
      links: [],
    }),
  },
  loadingComponent: ParcelLoadingSkeleton,
  errorComponent: ParcelErrorCard,
  emptyComponent: ParcelEmptyCard,
  // parcel data flows through V-World which is auth-gated upstream + the
  // /listings map (only entry point) is auth-gated, so true is the safe pick.
  // Spec § 6 example shows false; we diverge consciously here.
  authGate: { required: true },
  i18nNamespace: "panels.parcel",
  telemetryAttrs: (entry) => ({ pnu: entry.id }),
});
