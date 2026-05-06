/**
 * SP6-iii: `/listings/[id]` 매물 상세 페이지.
 *
 * server component — initial fetch 후 client `ListingDetail` 에 hydrate.
 * RBAC 거부 (404) → not-found UI.
 */

import { isHTTPError } from "ky";
import { notFound } from "next/navigation";

import { ListingDetail } from "@/components/listings/listing-detail";
import { fetchListingDetail } from "@/lib/listings/api";

interface PageProps {
  params: Promise<{ id: string }>;
}

export default async function ListingDetailPage({
  params,
}: PageProps): Promise<React.ReactElement> {
  const { id } = await params;
  try {
    const data = await fetchListingDetail(id);
    return <ListingDetail data={data} />;
  } catch (e) {
    if (isHTTPError(e) && e.response.status === 404) {
      notFound();
    }
    throw e;
  }
}
