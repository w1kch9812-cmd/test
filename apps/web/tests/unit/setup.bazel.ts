import * as matchers from "@testing-library/jest-dom/matchers";
import { expect } from "vitest";

expect.extend(matchers);

// Keep this in sync with setup.ts without importing the vitest peer wrapper.
process.env.ZITADEL_ISSUER = "http://localhost:8443";
process.env.ZITADEL_CLIENT_ID = "test-client";
process.env.ZITADEL_AUDIENCE = "test-client";
process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
process.env.REDIS_URL = "redis://localhost:6379";
process.env.SESSION_SECRET = "test-secret-placeholder-32-chars-x";
process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID = "test-naver-client";
process.env.PLATFORM_CORE_WEBHOOK_SECRET = "test-platform-core-webhook-secret-32-valid";
