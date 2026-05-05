import { z } from "zod";

/**
 * Client + server 공통: NEXT_PUBLIC_* 만 client bundle 에 inline됨.
 */
const PublicEnvSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: z.string().url().default("http://localhost:8080"),
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: z.string().min(1).default("naver-maps-placeholder"),
});

/**
 * Server 전용: server-only secrets. Browser 에서 access 시 undefined.
 */
const ServerEnvSchema = PublicEnvSchema.extend({
  ZITADEL_ISSUER: z.string().url(),
  ZITADEL_CLIENT_ID: z.string().min(1),
  ZITADEL_AUDIENCE: z.string().min(1),
  ZITADEL_REDIRECT_URI: z.string().url(),
  REDIS_URL: z.string().url(),
  SESSION_SECRET: z.string().min(32),
});

const isServer = typeof window === "undefined";
const Schema = isServer ? ServerEnvSchema : PublicEnvSchema;

const parsed = Schema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID,
  ZITADEL_ISSUER: process.env.ZITADEL_ISSUER,
  ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID,
  ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE,
  ZITADEL_REDIRECT_URI: process.env.ZITADEL_REDIRECT_URI,
  REDIS_URL: process.env.REDIS_URL,
  SESSION_SECRET: process.env.SESSION_SECRET,
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
