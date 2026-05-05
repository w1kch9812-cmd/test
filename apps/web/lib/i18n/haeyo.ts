const RTF = new Intl.RelativeTimeFormat("ko", { numeric: "auto" });

export function formatRelativeTime(date: Date | string): string {
  const target = typeof date === "string" ? new Date(date) : date;
  const diff = Math.floor((target.getTime() - Date.now()) / 1000);

  if (Math.abs(diff) < 60) return RTF.format(Math.floor(diff), "second");
  if (Math.abs(diff) < 3600) return RTF.format(Math.floor(diff / 60), "minute");
  if (Math.abs(diff) < 86400) return RTF.format(Math.floor(diff / 3600), "hour");
  if (Math.abs(diff) < 2592000) return RTF.format(Math.floor(diff / 86400), "day");
  if (Math.abs(diff) < 31536000) return RTF.format(Math.floor(diff / 2592000), "month");
  return RTF.format(Math.floor(diff / 31536000), "year");
}

const KRW_FORMAT = new Intl.NumberFormat("ko-KR", {
  style: "currency",
  currency: "KRW",
  maximumFractionDigits: 0,
});

export function formatKrw(amount: number): string {
  return KRW_FORMAT.format(amount);
}

const NUMBER_FORMAT = new Intl.NumberFormat("ko-KR");

export function formatNumber(n: number): string {
  return NUMBER_FORMAT.format(n);
}

export function formatAreaM2(m2: number): string {
  return `${NUMBER_FORMAT.format(Math.round(m2 * 10) / 10)}m²`;
}

export function formatCount(n: number, unit: string): string {
  return `${NUMBER_FORMAT.format(n)}${unit}`;
}
