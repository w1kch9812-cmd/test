"use client";

/**
 * SP6-iv: 매물 등록 폼.
 *
 * react-hook-form + zod resolver. cross-field validation
 * (`createListingSchema.superRefine`) 가 transaction_type 별
 * deposit/monthly_rent 필요 여부 강제. server-side 도메인이 SSOT — client 가
 * 통과해도 server 가 거부할 수 있고, 그 ProblemDetails 가 토스트로 노출.
 *
 * PhotoUploader 는 SP4-iii-e R2 통합 후 (FU 56). 현재 폼은 매물 메타만 등록.
 */

import { Button, Input, Label } from "@gongzzang/ui";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { isHTTPError } from "ky";
import type { Route } from "next";
import { useRouter } from "next/navigation";
import { useTranslations } from "next-intl";
import {
  type FieldErrors,
  type UseFormRegister,
  type UseFormSetError,
  useForm,
} from "react-hook-form";
import { toast } from "sonner";

import { createListing } from "@/lib/listings/mutations";
import {
  CONTACT_VISIBILITIES,
  type CreateListingFormValues,
  createListingSchema,
  LISTING_TYPES,
  TRANSACTION_TYPES,
} from "@/lib/listings/schema";
import { ROUTES } from "@/lib/routes";

interface ProblemDetailsBody {
  title: string;
  detail: string | null;
  type: string | null;
}

interface PricingFieldsProps {
  errors: FieldErrors<CreateListingFormValues>;
  register: UseFormRegister<CreateListingFormValues>;
  showDeposit: boolean;
  showMonthlyRent: boolean;
  t: (key: string) => string;
  tErr: (message: string | undefined) => string | undefined;
}

function nullableNumber(value: unknown): number | null {
  return value === "" || value === null ? null : Number(value);
}

function PricingFields({
  errors,
  register,
  showDeposit,
  showMonthlyRent,
  t,
  tErr,
}: PricingFieldsProps) {
  return (
    <div className="grid grid-cols-3 gap-4">
      <div className="space-y-2">
        <Label htmlFor="price_krw">{t("labels.price")}</Label>
        <Input
          id="price_krw"
          type="number"
          inputMode="numeric"
          {...register("price_krw", { valueAsNumber: true })}
        />
        {errors.price_krw ? (
          <p className="text-sm text-red-600">{tErr(errors.price_krw?.message)}</p>
        ) : null}
      </div>

      {showDeposit ? (
        <div className="space-y-2">
          <Label htmlFor="deposit_krw">{t("labels.deposit")}</Label>
          <Input
            id="deposit_krw"
            type="number"
            inputMode="numeric"
            {...register("deposit_krw", {
              valueAsNumber: true,
              setValueAs: nullableNumber,
            })}
          />
          {errors.deposit_krw ? (
            <p className="text-sm text-red-600">{tErr(errors.deposit_krw?.message)}</p>
          ) : null}
        </div>
      ) : null}

      {showMonthlyRent ? (
        <div className="space-y-2">
          <Label htmlFor="monthly_rent_krw">{t("labels.monthlyRent")}</Label>
          <Input
            id="monthly_rent_krw"
            type="number"
            inputMode="numeric"
            {...register("monthly_rent_krw", {
              valueAsNumber: true,
              setValueAs: nullableNumber,
            })}
          />
          {errors.monthly_rent_krw ? (
            <p className="text-sm text-red-600">{tErr(errors.monthly_rent_krw?.message)}</p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function translateFormError(
  translate: (key: string) => string,
  message: string | undefined,
): string | undefined {
  if (!message) return undefined;
  try {
    return translate(message);
  } catch {
    return message;
  }
}

function stringProperty(body: unknown, key: "title" | "detail" | "type"): string | null {
  if (typeof body !== "object" || body === null || !(key in body)) return null;
  const value = (body as Record<string, unknown>)[key];
  return typeof value === "string" ? value : null;
}

function parseProblemDetailsBody(body: unknown, fallbackTitle: string): ProblemDetailsBody {
  return {
    title: stringProperty(body, "title") ?? fallbackTitle,
    detail: stringProperty(body, "detail"),
    type: stringProperty(body, "type"),
  };
}

async function showCreateListingHttpError(
  error: unknown,
  fallbackTitle: string,
  setError: UseFormSetError<CreateListingFormValues>,
): Promise<void> {
  if (!isHTTPError(error)) {
    toast.error(fallbackTitle);
    return;
  }

  try {
    const problem = parseProblemDetailsBody(await error.response.json(), fallbackTitle);
    toast.error(problem.detail ? `${problem.title}: ${problem.detail}` : problem.title);
    if (problem.type?.includes("transaction-fields")) {
      setError("transaction_type", {
        message: problem.detail ?? problem.title,
      });
    }
  } catch {
    toast.error(fallbackTitle);
  }
}

export function ListingForm(): React.ReactElement {
  const router = useRouter();
  const t = useTranslations("listingForm");
  const tRoot = useTranslations();
  const tListingType = useTranslations("panels.listing.summary.type");
  const tTx = useTranslations("panels.listing.summary.transaction");
  const tVis = useTranslations("listingForm.contactVisibility");

  // zod schema 의 message 는 i18n key (예: 'listingForm.errors.pnuFormat').
  // 표시 시점에 root translator 로 변환. 키 누락 시 raw key 그대로 표시 (개발자에게 신호).
  const tErr = (msg: string | undefined): string | undefined => translateFormError(tRoot, msg);

  const LISTING_TYPE_LABELS: Record<(typeof LISTING_TYPES)[number], string> = {
    factory: tListingType("factory"),
    warehouse: tListingType("warehouse"),
    office: tListingType("office"),
    knowledge_industry_center: tListingType("knowledge_industry_center"),
    industrial_land: tListingType("industrial_land"),
    logistics_center: tListingType("logistics_center"),
  };
  const TRANSACTION_TYPE_LABELS: Record<(typeof TRANSACTION_TYPES)[number], string> = {
    sale: tTx("sale"),
    monthly_rent: tTx("monthly_rent"),
    jeonse: tTx("jeonse"),
  };
  const CONTACT_VISIBILITY_LABELS: Record<(typeof CONTACT_VISIBILITIES)[number], string> = {
    public: tVis("public"),
    login_required: tVis("login_required"),
    verified_only: tVis("verified_only"),
  };

  const {
    register,
    handleSubmit,
    watch,
    formState: { errors, isSubmitting },
    setError,
  } = useForm<CreateListingFormValues>({
    resolver: zodResolver(createListingSchema),
    defaultValues: {
      contact_visibility: "login_required",
      deposit_krw: null,
      monthly_rent_krw: null,
    },
    mode: "onBlur",
  });

  const transactionType = watch("transaction_type");
  const showDeposit = transactionType === "monthly_rent" || transactionType === "jeonse";
  const showMonthlyRent = transactionType === "monthly_rent";

  const mutation = useMutation({
    mutationFn: createListing,
    onSuccess(data) {
      toast.success(t("submitSuccess", { id: data.id }));
      router.push(ROUTES.listings.index as Route);
    },
    onError(error) {
      // RFC 7807 ProblemDetails 매핑 — server 가 client 검증 통과 후 거부 시.
      const fallback = t("submitErrorFallback");
      void showCreateListingHttpError(error, fallback, setError);
    },
  });

  return (
    <form
      onSubmit={handleSubmit((values) => mutation.mutate(values))}
      className="space-y-6"
      noValidate
    >
      <div className="space-y-2">
        <Label htmlFor="parcel_pnu">{t("labels.pnu")}</Label>
        <Input
          id="parcel_pnu"
          inputMode="numeric"
          maxLength={19}
          placeholder="1111010100100010000"
          {...register("parcel_pnu")}
        />
        {errors.parcel_pnu ? (
          <p className="text-sm text-red-600">{tErr(errors.parcel_pnu?.message)}</p>
        ) : null}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="listing_type">{t("labels.listingType")}</Label>
          <select
            id="listing_type"
            className="flex h-10 w-full rounded-[var(--radius-md)] border border-[var(--color-hairline)] bg-[var(--color-canvas)] px-3.5"
            {...register("listing_type")}
          >
            <option value="">{t("labels.selectPlaceholder")}</option>
            {LISTING_TYPES.map((t) => (
              <option key={t} value={t}>
                {LISTING_TYPE_LABELS[t]}
              </option>
            ))}
          </select>
          {errors.listing_type ? (
            <p className="text-sm text-red-600">{tErr(errors.listing_type?.message)}</p>
          ) : null}
        </div>

        <div className="space-y-2">
          <Label htmlFor="transaction_type">{t("labels.transactionType")}</Label>
          <select
            id="transaction_type"
            className="flex h-10 w-full rounded-[var(--radius-md)] border border-[var(--color-hairline)] bg-[var(--color-canvas)] px-3.5"
            {...register("transaction_type")}
          >
            <option value="">{t("labels.selectPlaceholder")}</option>
            {TRANSACTION_TYPES.map((t) => (
              <option key={t} value={t}>
                {TRANSACTION_TYPE_LABELS[t]}
              </option>
            ))}
          </select>
          {errors.transaction_type ? (
            <p className="text-sm text-red-600">{tErr(errors.transaction_type?.message)}</p>
          ) : null}
        </div>
      </div>

      <PricingFields
        errors={errors}
        register={register}
        showDeposit={showDeposit}
        showMonthlyRent={showMonthlyRent}
        t={t}
        tErr={tErr}
      />

      <div className="space-y-2">
        <Label htmlFor="area_m2">{t("labels.area")}</Label>
        <Input
          id="area_m2"
          type="number"
          step="0.01"
          {...register("area_m2", { valueAsNumber: true })}
        />
        {errors.area_m2 ? (
          <p className="text-sm text-red-600">{tErr(errors.area_m2?.message)}</p>
        ) : null}
      </div>

      <div className="space-y-2">
        <Label htmlFor="title">{t("labels.title")}</Label>
        <Input id="title" maxLength={200} {...register("title")} />
        {errors.title ? (
          <p className="text-sm text-red-600">{tErr(errors.title?.message)}</p>
        ) : null}
      </div>

      <div className="space-y-2">
        <Label htmlFor="description">{t("labels.description")}</Label>
        <textarea
          id="description"
          rows={6}
          maxLength={5000}
          className="flex w-full rounded-[var(--radius-md)] border border-[var(--color-hairline)] bg-[var(--color-canvas)] p-3.5"
          {...register("description")}
        />
        {errors.description ? (
          <p className="text-sm text-red-600">{tErr(errors.description?.message)}</p>
        ) : null}
      </div>

      <div className="space-y-2">
        <Label htmlFor="contact_visibility">{t("labels.contactVisibilityLabel")}</Label>
        <select
          id="contact_visibility"
          className="flex h-10 w-full rounded-[var(--radius-md)] border border-[var(--color-hairline)] bg-[var(--color-canvas)] px-3.5"
          {...register("contact_visibility")}
        >
          {CONTACT_VISIBILITIES.map((v) => (
            <option key={v} value={v}>
              {CONTACT_VISIBILITY_LABELS[v]}
            </option>
          ))}
        </select>
      </div>

      <p className="text-sm text-[var(--color-muted)]">{t("photoNotice")}</p>

      <Button type="submit" disabled={isSubmitting || mutation.isPending}>
        {isSubmitting || mutation.isPending ? t("submit.pending") : t("submit.idle")}
      </Button>
    </form>
  );
}
