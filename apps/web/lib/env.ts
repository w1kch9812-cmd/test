import { z } from "zod";

const EnvSchema = z.object({
  // SP6-foundation
  NEXT_PUBLIC_API_BASE_URL: z.string().url().default("http://localhost:8080"),

  // SP6-i: Zitadel OIDC
  ZITADEL_ISSUER: z.string().url(),
  ZITADEL_CLIENT_ID: z.string().min(1),
  ZITADEL_AUDIENCE: z.string().min(1),
  ZITADEL_REDIRECT_URI: z.string().url(),

  // SP6-i: Redis session + ratelimit
  REDIS_URL: z.string().url(),

  // SP6-i: cookie sealing (iron-session 호환 길이 32+)
  SESSION_SECRET: z.string().min(32),
});

const parsed = EnvSchema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
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

export const env = parsed.data;
export type Env = z.infer<typeof EnvSchema>;
