/**
 * SP6-iv: createListingSchema cross-field invariant 단위 테스트.
 *
 * server-side 도메인 invariant `V003_01` (transaction_type 별 deposit/monthly_rent
 * 필요 여부) 와 동기화 검증. 차이 발생 시 backend 가 진실 — 본 테스트는
 * client-side UX assist 정확성만.
 */

import { describe, expect, it } from "vitest";
import { createListingSchema } from "@/lib/listings/schema";

const baseValid = {
  parcel_pnu: "1111010100100010000",
  listing_type: "factory",
  transaction_type: "sale",
  price_krw: 500_000_000,
  deposit_krw: null,
  monthly_rent_krw: null,
  area_m2: 250.5,
  title: "강남 공장 매물",
  description: "위치 좋아요.",
  contact_visibility: "login_required",
} as const;

describe("createListingSchema cross-field invariant", () => {
  it("sale: deposit/monthly_rent 모두 null 이면 통과", () => {
    const result = createListingSchema.safeParse(baseValid);
    expect(result.success).toBe(true);
  });

  it("sale + deposit 입력 시 거부", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      deposit_krw: 50_000_000,
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const fields = result.error.issues.map((i: { path: (string | number)[] }) =>
        i.path.join("."),
      );
      expect(fields).toContain("deposit_krw");
    }
  });

  it("jeonse: deposit 만 있어야 통과", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      transaction_type: "jeonse",
      deposit_krw: 200_000_000,
      monthly_rent_krw: null,
    });
    expect(result.success).toBe(true);
  });

  it("jeonse + monthly_rent 입력 시 거부", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      transaction_type: "jeonse",
      deposit_krw: 200_000_000,
      monthly_rent_krw: 100_000,
    });
    expect(result.success).toBe(false);
  });

  it("monthly_rent: deposit + monthly_rent 둘 다 있어야 통과", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      transaction_type: "monthly_rent",
      deposit_krw: 50_000_000,
      monthly_rent_krw: 1_500_000,
    });
    expect(result.success).toBe(true);
  });

  it("monthly_rent + deposit 만 있고 monthly_rent 없으면 거부", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      transaction_type: "monthly_rent",
      deposit_krw: 50_000_000,
      monthly_rent_krw: null,
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      const fields = result.error.issues.map((i: { path: (string | number)[] }) =>
        i.path.join("."),
      );
      expect(fields).toContain("monthly_rent_krw");
    }
  });

  it("PNU 19자리 아니면 거부", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      parcel_pnu: "12345",
    });
    expect(result.success).toBe(false);
  });

  it("title 200자 초과 시 거부", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      title: "가".repeat(201),
    });
    expect(result.success).toBe(false);
  });

  it("price_krw 음수 거부", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      price_krw: -1,
    });
    expect(result.success).toBe(false);
  });

  it("area_m2 0 거부 (양수 invariant)", () => {
    const result = createListingSchema.safeParse({
      ...baseValid,
      area_m2: 0,
    });
    expect(result.success).toBe(false);
  });
});
