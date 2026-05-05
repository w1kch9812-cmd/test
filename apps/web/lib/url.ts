/**
 * returnTo 값을 same-origin path 로만 제한.
 * 외부 URL / scheme 시도 → "/profile" 로 fallback.
 * Open redirect 차단 (security-critical).
 */
export function sanitizeReturnTo(value: string | null | undefined): string {
  if (!value || typeof value !== "string") return "/profile";
  // path 만 허용 — "/foo", "/foo?bar=1", "/foo#x"
  if (!value.startsWith("/")) return "/profile";
  // protocol-relative URL 차단 ("//evil.com")
  if (value.startsWith("//")) return "/profile";
  // backslash 우회 차단 ("/\evil.com")
  if (value.includes("\\")) return "/profile";
  return value;
}
