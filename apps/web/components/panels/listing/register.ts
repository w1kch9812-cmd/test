// apps/web/components/panels/listing/register.ts
// Side-effect-only module: importing this file triggers defineKind('listing') once.
// No exports. T6 imports it from app/listings/page.tsx for registration.

import { fetchListingDetail, type ListingDetail } from "@/lib/listings/api";
import { defineKind, defineView } from "@/lib/panel/registry";
import { ListingEmptyCard, ListingErrorCard, ListingLoadingSkeleton } from "./skeletons";
import { ListingSummaryCard } from "./summary";

defineKind({
  kind: "listing",
  idPattern: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
  views: {
    summary: defineView<"listing", ListingDetail>({
      component: ListingSummaryCard,
      fetcher: (id) => fetchListingDetail(id),
      staleTime: 60_000,
      links: [],
    }),
  },
  loadingComponent: ListingLoadingSkeleton,
  errorComponent: ListingErrorCard,
  emptyComponent: ListingEmptyCard,
  authGate: { required: true },
  i18nNamespace: "panels.listing",
  telemetryAttrs: (entry) => ({ listing_id: entry.id }),
});
