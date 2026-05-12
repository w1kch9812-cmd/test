const TRILLION = 1_000_000_000_000n;
const HUNDRED_MILLION = 100_000_000n;
const TEN_THOUSAND = 10_000n;
const PYEONG_PER_M2 = 0.3025; // 1 평 = 3.305 m² → 1 m² ≈ 0.3025 평

// 한국 도메인 단위 표기 SSOT (한국 시장 한정 표준 — 영어 locale 추가 시 별도
// formatter 또는 locale switch 필요). 사용자 노출 단위는 본 const 외 다른 곳
// 에서 hardcode 금지.
const KRW_UNIT = "원";
const KRW_TRILLION = "조";
const KRW_HUNDRED_MILLION = "억";
const KRW_TEN_THOUSAND = "만";
const AREA_PYEONG = "평";
const AREA_M2 = "㎡";

/**
 * 한국 가격 표기 (1조 5,000억원 / 85억원 / 1억 2,345만원 / 5,000만원 / 800,000원).
 */
export function formatPriceKrw(value: number): string {
  if (value === 0) return `0${KRW_UNIT}`;
  const big = BigInt(Math.round(value));
  const trillions = big / TRILLION;
  const remainderAfterTrillions = big % TRILLION;
  const hundredMillions = remainderAfterTrillions / HUNDRED_MILLION;
  const remainderAfterHM = remainderAfterTrillions % HUNDRED_MILLION;
  const tenThousands = remainderAfterHM / TEN_THOUSAND;

  const parts: string[] = [];
  if (trillions > 0n) parts.push(`${trillions}${KRW_TRILLION}`);
  if (hundredMillions > 0n) {
    if (trillions > 0n) {
      parts.push(`${formatThousands(hundredMillions)}${KRW_HUNDRED_MILLION}${KRW_UNIT}`);
      return parts.join(" ");
    }
    parts.push(`${formatThousands(hundredMillions)}${KRW_HUNDRED_MILLION}`);
    if (tenThousands > 0n) {
      parts.push(`${formatThousands(tenThousands)}${KRW_TEN_THOUSAND}${KRW_UNIT}`);
    } else {
      parts[parts.length - 1] = `${parts[parts.length - 1]}${KRW_UNIT}`;
    }
    return parts.join(" ");
  }
  // 100만원 미만은 원 단위 직접 표기 (예: 800,000원)
  if (big >= 1_000_000n && tenThousands > 0n) {
    return `${formatThousands(tenThousands)}${KRW_TEN_THOUSAND}${KRW_UNIT}`;
  }
  return `${formatThousands(big)}${KRW_UNIT}`;
}

function formatThousands(n: bigint): string {
  return n.toLocaleString("ko-KR");
}

/** m² → 평 변환. */
export function m2ToPyeong(m2: number): number {
  return m2 * PYEONG_PER_M2;
}

/** "100.0평" 형식. */
export function formatAreaPyeong(m2: number): string {
  return `${m2ToPyeong(m2).toFixed(1)}${AREA_PYEONG}`;
}

/** "3,961㎡" 형식 (정수 + 천단위 콤마). */
export function formatAreaM2(m2: number): string {
  return `${Math.round(m2).toLocaleString("ko-KR")}${AREA_M2}`;
}

/** 한국 도메인 단위 — caller 가 직접 표기할 때 사용 (예: '5,000 원 · 100 ㎡'). */
export const UNITS = {
  krw: KRW_UNIT,
  pyeong: AREA_PYEONG,
  m2: AREA_M2,
} as const;
