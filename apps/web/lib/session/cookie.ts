export const SID_COOKIE_NAME = "__Host-sid";
export const TEMP_COOKIE_NAME = "__Host-auth-tmp";

export function setSidCookie(sid: string, maxAgeSec: number): string {
  // __Host- prefix 는 Domain 속성 금지, Path=/ 필수, Secure 필수
  return [
    `${SID_COOKIE_NAME}=${sid}`,
    "Secure",
    "HttpOnly",
    "SameSite=Strict",
    "Path=/",
    `Max-Age=${maxAgeSec}`,
    "Partitioned",
  ].join("; ");
}

export function deleteSidCookie(): string {
  return [
    `${SID_COOKIE_NAME}=`,
    "Secure",
    "HttpOnly",
    "SameSite=Strict",
    "Path=/",
    "Max-Age=0",
    "Partitioned",
  ].join("; ");
}

export interface TempAuthState {
  code_verifier: string;
  state: string;
  nonce: string;
  return_to: string;
}

export function setTempCookie(payload: string, maxAgeSec: number): string {
  return [
    `${TEMP_COOKIE_NAME}=${payload}`,
    "Secure",
    "HttpOnly",
    "SameSite=Lax", // OAuth callback 은 cross-site GET 이라 Strict 불가
    "Path=/api/auth/",
    `Max-Age=${maxAgeSec}`,
  ].join("; ");
}

export function deleteTempCookie(): string {
  return [
    `${TEMP_COOKIE_NAME}=`,
    "Secure",
    "HttpOnly",
    "SameSite=Lax",
    "Path=/api/auth/",
    "Max-Age=0",
  ].join("; ");
}
