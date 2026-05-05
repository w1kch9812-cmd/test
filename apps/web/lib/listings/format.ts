const TRILLION = 1_000_000_000_000n;
const HUNDRED_MILLION = 100_000_000n;
const TEN_THOUSAND = 10_000n;
const PYEONG_PER_M2 = 0.3025; // 1 평 = 3.305 m² → 1 m² ≈ 0.3025 평

/**
 * 한국 가격 표기 (1조 5,000억원 / 85억원 / 1억 2,345만원 / 5,000만원 / 800,000원).
 */
export function formatPriceKrw(value: number): string {
  if (value === 0) return "0원";
  const big = BigInt(Math.round(value));
  const trillions = big / TRILLION;
  const remainderAfterTrillions = big % TRILLION;
  const hundredMillions = remainderAfterTrillions / HUNDRED_MILLION;
  const remainderAfterHM = remainderAfterTrillions % HUNDRED_MILLION;
  const tenThousands = remainderAfterHM / TEN_THOUSAND;

  const parts: string[] = [];
  if (trillions > 0n) parts.push(`${trillions}조`);
  if (hundredMillions > 0n) {
    if (trillions > 0n) {
      parts.push(`${formatThousands(hundredMillions)}억원`);
      return parts.join(" ");
    }
    parts.push(`${formatThousands(hundredMillions)}억`);
    if (tenThousands > 0n) parts.push(`${formatThousands(tenThousands)}만원`);
    else parts[parts.length - 1] = `${parts[parts.length - 1]}원`;
    return parts.join(" ");
  }
  // 100만원 미만은 원 단위 직접 표기 (예: 800,000원)
  if (big >= 1_000_000n && tenThousands > 0n) return `${formatThousands(tenThousands)}만원`;
  return `${formatThousands(big)}원`;
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
  return `${m2ToPyeong(m2).toFixed(1)}평`;
}

/** "3,961㎡" 형식 (정수 + 천단위 콤마). */
export function formatAreaM2(m2: number): string {
  return `${Math.round(m2).toLocaleString("ko-KR")}㎡`;
}
