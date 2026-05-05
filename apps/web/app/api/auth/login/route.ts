import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { buildAuthorizationUrl, generatePkceParams } from "@/lib/oidc";
import { setTempCookie } from "@/lib/session/cookie";

export async function POST(req: NextRequest) {
  const formData = await req.formData().catch(() => null);
  const returnTo = (formData?.get("returnTo") as string) ?? "/profile";

  const pkce = await generatePkceParams();
  const authUrl = buildAuthorizationUrl({
    issuer: env.ZITADEL_ISSUER,
    clientId: env.ZITADEL_CLIENT_ID,
    redirectUri: env.ZITADEL_REDIRECT_URI,
    scope: "openid profile email offline_access",
    code_challenge: pkce.code_challenge,
    state: pkce.state,
    nonce: pkce.nonce,
  });

  const tmp = Buffer.from(
    JSON.stringify({
      code_verifier: pkce.code_verifier,
      state: pkce.state,
      nonce: pkce.nonce,
      return_to: returnTo,
    }),
  ).toString("base64url");

  return new NextResponse(null, {
    status: 302,
    headers: {
      Location: authUrl,
      "Set-Cookie": setTempCookie(tmp, 600),
    },
  });
}

export async function GET(req: NextRequest) {
  return POST(req);
}
