import * as oauth from "oauth4webapi";

export interface PkceParams {
  code_verifier: string;
  code_challenge: string;
  state: string;
  nonce: string;
}

export async function generatePkceParams(): Promise<PkceParams> {
  const code_verifier = oauth.generateRandomCodeVerifier();
  const code_challenge = await oauth.calculatePKCECodeChallenge(code_verifier);
  return {
    code_verifier,
    code_challenge,
    state: oauth.generateRandomState(),
    nonce: oauth.generateRandomNonce(),
  };
}

export interface AuthUrlInput {
  issuer: string;
  clientId: string;
  redirectUri: string;
  scope: string;
  code_challenge: string;
  state: string;
  nonce: string;
}

export function buildAuthorizationUrl(i: AuthUrlInput): string {
  const u = new URL(`${i.issuer}/oauth/v2/authorize`);
  u.searchParams.set("response_type", "code");
  u.searchParams.set("client_id", i.clientId);
  u.searchParams.set("redirect_uri", i.redirectUri);
  u.searchParams.set("scope", i.scope);
  u.searchParams.set("code_challenge", i.code_challenge);
  u.searchParams.set("code_challenge_method", "S256");
  u.searchParams.set("state", i.state);
  u.searchParams.set("nonce", i.nonce);
  return u.toString();
}

export interface EndSessionInput {
  issuer: string;
  idTokenHint: string;
  postLogoutRedirectUri: string;
}

export function buildEndSessionUrl(i: EndSessionInput): string {
  const u = new URL(`${i.issuer}/oidc/v1/end_session`);
  u.searchParams.set("id_token_hint", i.idTokenHint);
  u.searchParams.set("post_logout_redirect_uri", i.postLogoutRedirectUri);
  return u.toString();
}

let _as: oauth.AuthorizationServer | null = null;

export async function discoverAs(issuer: string): Promise<oauth.AuthorizationServer> {
  if (_as && _as.issuer === issuer) return _as;
  const resp = await oauth.discoveryRequest(new URL(issuer), {
    algorithm: "oidc",
  });
  _as = await oauth.processDiscoveryResponse(new URL(issuer), resp);
  return _as;
}

export interface TokenResult {
  access_token: string;
  refresh_token: string;
  id_token: string;
  expires_in: number;
  jti: string;
  sub: string;
  role: string;
}

export async function exchangeCode(input: {
  issuer: string;
  clientId: string;
  redirectUri: string;
  code: string;
  code_verifier: string;
  expectedNonce: string;
}): Promise<TokenResult> {
  const as = await discoverAs(input.issuer);
  // Public client (PKCE) — no client secret, use None() auth method
  const client: oauth.Client = { client_id: input.clientId };
  const callbackParams = new URLSearchParams({ code: input.code });
  const resp = await oauth.authorizationCodeGrantRequest(
    as,
    client,
    oauth.None(),
    callbackParams,
    input.redirectUri,
    input.code_verifier,
  );
  // v3: processAuthorizationCodeResponse handles OIDC when id_token is present
  const result = await oauth.processAuthorizationCodeResponse(as, client, resp, {
    expectedNonce: input.expectedNonce,
    requireIdToken: true,
  });
  const idClaims = oauth.getValidatedIdTokenClaims(result);
  if (!idClaims) {
    throw new Error("oidc error: id_token claims missing");
  }
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token ?? "",
    id_token: result.id_token ?? "",
    expires_in: result.expires_in ?? 300,
    jti: (idClaims.jti as string | undefined) ?? idClaims.sub,
    sub: idClaims.sub,
    role: extractRole(idClaims),
  };
}

function extractRole(claims: Record<string, unknown>): string {
  // Zitadel role assertion: urn:zitadel:iam:org:project:roles
  const roleClaim = claims["urn:zitadel:iam:org:project:roles"];
  if (typeof roleClaim === "object" && roleClaim !== null) {
    const first = Object.keys(roleClaim)[0];
    if (first) return first;
  }
  return "Buyer"; // default safe role
}

export async function refreshTokens(input: {
  issuer: string;
  clientId: string;
  refresh_token: string;
}): Promise<TokenResult> {
  const as = await discoverAs(input.issuer);
  const client: oauth.Client = { client_id: input.clientId };
  const resp = await oauth.refreshTokenGrantRequest(as, client, oauth.None(), input.refresh_token);
  const result = await oauth.processRefreshTokenResponse(as, client, resp);
  const idClaims = oauth.getValidatedIdTokenClaims(result);
  if (!idClaims) {
    throw new Error("refresh error: id_token claims missing");
  }
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token ?? "",
    id_token: result.id_token ?? "",
    expires_in: result.expires_in ?? 300,
    jti: (idClaims.jti as string | undefined) ?? idClaims.sub,
    sub: idClaims.sub,
    role: extractRole(idClaims),
  };
}
