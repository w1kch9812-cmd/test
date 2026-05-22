// apps/web/components/panels/listing/register.ts
// Side-effect-only module: importing this file triggers defineKind('listing') once.
// No exports. T6 imports it from app/listings/page.tsx for registration.

import { LISTING_ID_PATTERN } from "@/lib/identity/patterns";
import { fetchListingDetail, type ListingDetail } from "@/lib/listings/api";
import { defineKind, defineView } from "@/lib/panel/registry";
import { ListingEmptyCard, ListingErrorCard, ListingLoadingSkeleton } from "./skeletons";
import { ListingSummaryCard } from "./summary";

defineKind({
  kind: "listing",
  idPattern: LISTING_ID_PATTERN,
  views: {
    summary: defineView<"listing", ListingDetail>({
      component: ListingSummaryCard,
      fetcher: (id, signal) => fetchListingDetail(id, signal),
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
