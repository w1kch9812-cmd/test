/**
 * SP6-iv: 매물 등록/수정 TanStack Query mutations.
 *
 * `POST /listings`, `PATCH /listings/:id`, `POST /listings/:id/submit-for-review`
 * 등을 wrapping. Server-side 도메인이 거부 시 RFC 7807 ProblemDetails 가 ky
 * `HTTPError` 로 throw — caller 가 detail 파싱.
 */

import { apiProxyClient } from "@/lib/api/api-proxy-client.generated";
import {
  type CreateListingFormValues,
  type CreateListingResponse,
  CreateListingResponseSchema,
} from "@/lib/listings/schema";

/**
 * 새 매물 등록 (`POST /listings`).
 *
 * Server payload 형식 = backend `CreateListingRequest`. price/deposit/monthly_rent
 * 모두 `*_krw` 정수, deposit/monthly_rent 는 null 가능.
 */
export async function createListing(
  values: CreateListingFormValues,
): Promise<CreateListingResponse> {
  const body = {
    parcel_pnu: values.parcel_pnu,
    listing_type: values.listing_type,
    transaction_type: values.transaction_type,
    price_krw: values.price_krw,
    deposit_krw: values.deposit_krw,
    monthly_rent_krw: values.monthly_rent_krw,
    area_m2: values.area_m2,
    title: values.title,
    description: values.description,
    contact_visibility: values.contact_visibility,
  };

  const json = await apiProxyClient.listingsCollectionCreate.postJson<unknown>({ json: body });
  return CreateListingResponseSchema.parse(json);
}

/**
 * `POST /listings/:id/submit-for-review`. Draft → PendingReview.
 * Returns new `{ id, version, status }`.
 */
// ── SP6-iii: 북마크 toggle ─────────────────────────────────────────────

/**
 * `POST /listings/:id/bookmark` — 멱등 UPSERT.
 * 같은 사용자/매물 두번째 호출은 note 갱신 (서버 멱등 design).
 */
export async function addBookmark(listingId: string, note?: string): Promise<void> {
  await apiProxyClient.listingBookmark.postJson<unknown>(
    { id: listingId },
    { json: { note: note ?? null } },
  );
}

/**
 * `DELETE /listings/:id/bookmark` — 멱등 (이미 없어도 200).
 */
export async function removeBookmark(listingId: string): Promise<void> {
  await apiProxyClient.listingBookmark.delete({ id: listingId });
}

export async function submitForReview(
  listingId: string,
): Promise<{ id: string; version: number; status: string }> {
  const json = await apiProxyClient.listingSubmitForReview.postJson<unknown>({ id: listingId });

  // Keep this narrow response check at the API boundary.
  if (
    typeof json !== "object" ||
    json === null ||
    typeof (json as Record<string, unknown>).id !== "string" ||
    typeof (json as Record<string, unknown>).version !== "number" ||
    typeof (json as Record<string, unknown>).status !== "string"
  ) {
    throw new Error("invalid submit-for-review response");
  }
  return json as { id: string; version: number; status: string };
}
