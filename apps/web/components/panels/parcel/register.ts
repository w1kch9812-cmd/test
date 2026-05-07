// apps/web/components/panels/parcel/register.ts
import type { ComponentType } from "react";
import { fetchBuildings } from "@/lib/api/buildings";
import { fetchParcel } from "@/lib/api/parcels";
import { fetchListings } from "@/lib/listings/api";
import { defineKind } from "@/lib/panel/registry";
import type { PanelStackEntry } from "@/lib/panel/types";
import { ParcelBuildingsCard } from "./buildings";
import { ParcelListingsCard } from "./listings";
import { ParcelEmptyCard, ParcelErrorCard, ParcelLoadingSkeleton } from "./skeletons";
import { ParcelSummaryCard } from "./summary";

// Registry stores components with `data: unknown` (TData defaults to unknown
// in PanelViewDefinition). Each kind's view components are zod-parsed before
// reaching them at runtime — see fetchParcel/fetchBuildings/fetchListings —
// but the registry's invariant generic forces a structural cast at registration.
type ParcelEntry = Extract<PanelStackEntry, { kind: "parcel" }>;
type ParcelViewComponent = ComponentType<{ entry: ParcelEntry; data: unknown }>;

defineKind({
  kind: "parcel",
  idPattern: /^\d{19}$/,
  views: {
    summary: {
      component: ParcelSummaryCard as unknown as ParcelViewComponent,
      fetcher: (id) => fetchParcel(id),
      staleTime: 5 * 60_000,
      links: [],
    },
    buildings: {
      component: ParcelBuildingsCard as unknown as ParcelViewComponent,
      fetcher: (id) => fetchBuildings(id),
      staleTime: 5 * 60_000,
      links: [],
    },
    listings: {
      component: ParcelListingsCard as unknown as ParcelViewComponent,
      // T6 will move pnu to top-level fetchListings input + remove from filters;
      // until then, set on filters.
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
            pnu: id,
            adminCode: undefined,
            landUseType: undefined,
          },
        }),
      staleTime: 60_000,
      links: [],
    },
  },
  loadingComponent: ParcelLoadingSkeleton,
  errorComponent: ParcelErrorCard,
  emptyComponent: ParcelEmptyCard,
  authGate: { required: true },
  i18nNamespace: "panels.parcel",
  telemetryAttrs: (entry) => ({ pnu: entry.id }),
});
