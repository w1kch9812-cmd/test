/**
 * SP6-iv: 매물 등록/수정 zod schema.
 *
 * Server-side 도메인이 SSOT. 본 schema 는 form UX assist (즉각 피드백) — 통과해도
 * server 가 거부할 수 있음 (도메인 invariant 우선). 차이 발견 시 backend 가 진실.
 *
 * Cross-field invariant `V003_01`:
 * - sale: deposit/monthly_rent 모두 null
 * - jeonse: deposit Some, monthly_rent null
 * - monthly_rent: deposit + monthly_rent 모두 Some
 *
 * 도메인 backend `crates/domain/core/listing/src/entity.rs` 의
 * `try_new_draft` / `update_editable_fields` 와 동기화 필요. utoipa 자동 생성
 * (FU 55) 까지는 manual fork.
 */

import { z } from "zod";

export const LISTING_TYPES = [
  "factory",
  "warehouse",
  "office",
  "knowledge_industry_center",
  "industrial_land",
  "logistics_center",
] as const;

export const TRANSACTION_TYPES = ["sale", "monthly_rent", "jeonse"] as const;

export const CONTACT_VISIBILITIES = ["public", "login_required", "verified_only"] as const;

const PNU_REGEX = /^\d{19}$/;

// zod 4: errorMap → `error` callback, invalid_type_error 제거 (메시지는 fallback
// custom validator 또는 issue.message 처리). enum 메시지는 default 가 충분.
const baseListingFields = z.object({
  parcel_pnu: z.string().regex(PNU_REGEX, "PNU 는 19자리 숫자여야 해요"),
  listing_type: z.enum(LISTING_TYPES),
  transaction_type: z.enum(TRANSACTION_TYPES),
  price_krw: z.number().int().positive("가격은 0 보다 커야 해요"),
  deposit_krw: z.number().int().positive().nullable(),
  monthly_rent_krw: z.number().int().positive().nullable(),
  area_m2: z.number().positive("면적은 0 보다 커야 해요"),
  title: z.string().min(1, "제목을 입력해 주세요").max(200, "제목은 200자 이하여야 해요"),
  description: z.string().max(5000, "설명은 5000자 이하여야 해요"),
  // 폼 default 는 react-hook-form 의 defaultValues 가 셋팅 — schema 의 default
  // 는 input/output type 불일치 (zod 4) 로 react-hook-form Resolver 와 충돌.
  contact_visibility: z.enum(CONTACT_VISIBILITIES),
  geom_point: z
    .object({
      lng: z.number().gte(-180).lte(180),
      lat: z.number().gte(-90).lte(90),
    })
    .nullable()
    .optional(),
});

/**
 * Cross-field invariant — 거래 유형 별 deposit/monthly_rent 필요 여부.
 * domain `crates/domain/core/shared-kernel/src/transaction_type.rs` 와 동기.
 */
function refineTransactionFields(
  data: z.infer<typeof baseListingFields>,
  ctx: z.RefinementCtx,
): void {
  const { transaction_type, deposit_krw, monthly_rent_krw } = data;
  if (transaction_type === "sale") {
    if (deposit_krw !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["deposit_krw"],
        message: "매매 거래는 보증금이 없어야 해요",
      });
    }
    if (monthly_rent_krw !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthly_rent_krw"],
        message: "매매 거래는 월세가 없어야 해요",
      });
    }
  } else if (transaction_type === "jeonse") {
    if (deposit_krw === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["deposit_krw"],
        message: "전세는 보증금이 필요해요",
      });
    }
    if (monthly_rent_krw !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthly_rent_krw"],
        message: "전세는 월세가 없어야 해요",
      });
    }
  } else if (transaction_type === "monthly_rent") {
    if (deposit_krw === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["deposit_krw"],
        message: "월세는 보증금이 필요해요",
      });
    }
    if (monthly_rent_krw === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthly_rent_krw"],
        message: "월세는 월세 금액이 필요해요",
      });
    }
  }
}

export const createListingSchema = baseListingFields.superRefine(refineTransactionFields);

export type CreateListingFormValues = z.infer<typeof createListingSchema>;

export const CreateListingResponseSchema = z.object({
  id: z.string().regex(/^lst_[0-9A-HJKMNP-TV-Z]{26}$/),
  version: z.number().int().positive(),
});

export type CreateListingResponse = z.infer<typeof CreateListingResponseSchema>;
