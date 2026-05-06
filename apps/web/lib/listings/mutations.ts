/**
 * SP6-iv: 매물 등록/수정 TanStack Query mutations.
 *
 * `POST /listings`, `PATCH /listings/:id`, `POST /listings/:id/submit-for-review`
 * 등을 wrapping. Server-side 도메인이 거부 시 RFC 7807 ProblemDetails 가 ky
 * `HTTPError` 로 throw — caller 가 detail 파싱.
 */

import { api } from "@/lib/api";
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
    geom_point: values.geom_point ?? null,
  };

  const json = await api.post("listings", { json: body }).json<unknown>();
  return CreateListingResponseSchema.parse(json);
}

/**
 * `POST /listings/:id/submit-for-review`. Draft → PendingReview.
 * Returns new `{ id, version, status }`.
 */
export async function submitForReview(
  listingId: string,
): Promise<{ id: string; version: number; status: string }> {
  const json = await api.post(`listings/${listingId}/submit-for-review`).json<unknown>();

  // 임시 인라인 schema — 별도 파일로 분리는 FU 56 (편집 + revise + photo 풀 mutation 묶음).
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
