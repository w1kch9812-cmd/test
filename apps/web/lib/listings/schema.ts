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
import { LISTING_ID_PATTERN } from "@/lib/identity/patterns";

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
//
// i18n SSOT: message 는 *i18n key* (예: 'listingForm.errors.pnuFormat'). caller
// (form component) 가 `errors.[field].message` 표시 시 `useTranslations` 로
// translate. apps/web/lib/i18n/ko.json 의 listingForm.errors.* 와 동기화.
const baseListingFields = z.object({
  parcel_pnu: z.string().regex(PNU_REGEX, "listingForm.errors.pnuFormat"),
  listing_type: z.enum(LISTING_TYPES),
  transaction_type: z.enum(TRANSACTION_TYPES),
  price_krw: z.number().int().positive("listingForm.errors.priceMin"),
  deposit_krw: z.number().int().positive().nullable(),
  monthly_rent_krw: z.number().int().positive().nullable(),
  area_m2: z.number().positive("listingForm.errors.areaMin"),
  title: z
    .string()
    .min(1, "listingForm.errors.titleRequired")
    .max(200, "listingForm.errors.titleMax"),
  description: z.string().max(5000, "listingForm.errors.descriptionMax"),
  // 폼 default 는 react-hook-form 의 defaultValues 가 셋팅 — schema 의 default
  // 는 input/output type 불일치 (zod 4) 로 react-hook-form Resolver 와 충돌.
  contact_visibility: z.enum(CONTACT_VISIBILITIES),
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
        message: "listingForm.errors.saleDepositForbidden",
      });
    }
    if (monthly_rent_krw !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthly_rent_krw"],
        message: "listingForm.errors.saleMonthlyRentForbidden",
      });
    }
  } else if (transaction_type === "jeonse") {
    if (deposit_krw === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["deposit_krw"],
        message: "listingForm.errors.jeonseDepositRequired",
      });
    }
    if (monthly_rent_krw !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthly_rent_krw"],
        message: "listingForm.errors.jeonseMonthlyRentForbidden",
      });
    }
  } else if (transaction_type === "monthly_rent") {
    if (deposit_krw === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["deposit_krw"],
        message: "listingForm.errors.monthlyRentDepositRequired",
      });
    }
    if (monthly_rent_krw === null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthly_rent_krw"],
        message: "listingForm.errors.monthlyRentMonthlyRequired",
      });
    }
  }
}

export const createListingSchema = baseListingFields.superRefine(refineTransactionFields);

export type CreateListingFormValues = z.infer<typeof createListingSchema>;

export const CreateListingResponseSchema = z.object({
  id: z.string().regex(LISTING_ID_PATTERN),
  version: z.number().int().positive(),
});

export type CreateListingResponse = z.infer<typeof CreateListingResponseSchema>;
