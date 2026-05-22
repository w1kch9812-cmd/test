import { z } from "zod";

/**
 * Client + server 공통: NEXT_PUBLIC_* 만 client bundle 에 inline됨.
 */
const optionalUrl = z.preprocess(
  (value) => (typeof value === "string" && value.trim() === "" ? undefined : value),
  z.string().url().optional(),
);

const forbiddenPublicClientIds = new Set(["naver-maps-placeholder", "your_naver_client_id_here"]);
const devInternalAuthSecret = "dev-internal-auth-must-be-shared";
const forbiddenProductionSessionSecrets = new Set([
  "change-me-to-random-32-byte-base64-string-aaaaaaaaaa",
  "ci-placeholder-secret-32-bytes-padding-ok",
  "test-secret-placeholder-32-chars-x",
]);
const forbiddenProductionZitadelIdentifiers = new Set([
  "ci-placeholder",
  "demo-client",
  "placeholder",
  "test-client",
  "x",
  "your_zitadel_client_id_here",
]);
const isProduction = process.env.NODE_ENV === "production";
const loopbackHostnames = new Set(["127.0.0.1", "::1", "[::1]", "localhost"]);

const isProductionPublicUrl = (value: string) => {
  if (!isProduction) {
    return true;
  }
  const parsedUrl = new URL(value);
  return (
    parsedUrl.protocol === "https:" && !loopbackHostnames.has(parsedUrl.hostname.toLowerCase())
  );
};

const productionPublicUrlMessage = "must use a public https URL in production";

const requiredUrl = z.string().url();

const requiredProductionPublicUrl = requiredUrl.refine(isProductionPublicUrl, {
  message: productionPublicUrlMessage,
});

const requiredPublicClientId = z
  .string()
  .trim()
  .min(1)
  .refine((value) => !forbiddenPublicClientIds.has(value), {
    message: "must be configured explicitly",
  });

const requiredInternalAuthSecret = z
  .string()
  .trim()
  .min(16)
  .refine((value) => !isProduction || value !== devInternalAuthSecret, {
    message: "must be configured explicitly in production",
  });

const requiredSessionSecret = z
  .string()
  .trim()
  .min(32)
  .refine((value) => !isProduction || !forbiddenProductionSessionSecrets.has(value), {
    message: "must be configured explicitly in production",
  });

const requiredZitadelIdentifier = z
  .string()
  .trim()
  .min(1)
  .refine((value) => !isProduction || !forbiddenProductionZitadelIdentifiers.has(value), {
    message: "must be configured explicitly in production",
  });

const publicApiBaseUrl = isProduction
  ? requiredProductionPublicUrl
  : requiredUrl.default("http://localhost:8080");
const platformCoreBaseUrl = isProduction ? requiredProductionPublicUrl : optionalUrl;

const PublicEnvSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: publicApiBaseUrl,
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: requiredPublicClientId,
  NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: platformCoreBaseUrl,
  NEXT_PUBLIC_TILES_MANIFEST_URL: optionalUrl,
});

/**
 * Server 전용: server-only secrets. Browser 에서 access 시 undefined.
 */
const ServerEnvSchema = PublicEnvSchema.extend({
  ZITADEL_ISSUER: isProduction ? requiredProductionPublicUrl : requiredUrl,
  ZITADEL_CLIENT_ID: requiredZitadelIdentifier,
  ZITADEL_AUDIENCE: requiredZitadelIdentifier,
  ZITADEL_REDIRECT_URI: isProduction ? requiredProductionPublicUrl : requiredUrl,
  REDIS_URL: z.string().url(),
  SESSION_SECRET: requiredSessionSecret,
  // audit 2026-05-08: services/api 의 /internal/auth/event shared secret. Rust API
  // 와 *동일 값* 공유 — 미설정 시 401 (audit log 누락만, 서비스 정상). production
  // 은 Pulumi secret. dev 는 .env.local.
  INTERNAL_AUTH_SECRET: isProduction
    ? requiredInternalAuthSecret
    : requiredInternalAuthSecret.default(devInternalAuthSecret),
});

const isServer = typeof window === "undefined";
const Schema = isServer ? ServerEnvSchema : PublicEnvSchema;

const parsed = Schema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID,
  NEXT_PUBLIC_PLATFORM_CORE_BASE_URL: process.env.NEXT_PUBLIC_PLATFORM_CORE_BASE_URL,
  NEXT_PUBLIC_TILES_MANIFEST_URL: process.env.NEXT_PUBLIC_TILES_MANIFEST_URL,
  ZITADEL_ISSUER: process.env.ZITADEL_ISSUER,
  ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID,
  ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE,
  ZITADEL_REDIRECT_URI: process.env.ZITADEL_REDIRECT_URI,
  REDIS_URL: process.env.REDIS_URL,
  SESSION_SECRET: process.env.SESSION_SECRET,
  INTERNAL_AUTH_SECRET: process.env.INTERNAL_AUTH_SECRET,
});

if (!parsed.success) {
  throw new Error(
    `Invalid environment variables: ${JSON.stringify(parsed.error.flatten().fieldErrors)}`,
  );
}

/**
 * Server-only env 는 client bundle 에서 undefined.
 * server-only secrets 사용 코드는 Route Handler / Server Component 안에서만.
 */
export const env = parsed.data as z.infer<typeof ServerEnvSchema>;
export type Env = z.infer<typeof ServerEnvSchema>;
