import { describe, expect, it } from "vitest";
import { buildAuthorizationUrl, buildEndSessionUrl, generatePkceParams } from "@/lib/oidc";

describe("oidc helpers", () => {
  it("generatePkceParams returns code_verifier (43+ chars), code_challenge, state, nonce", async () => {
    const p = await generatePkceParams();
    expect(p.code_verifier.length).toBeGreaterThanOrEqual(43);
    expect(p.code_challenge.length).toBeGreaterThanOrEqual(43);
    expect(p.state.length).toBeGreaterThanOrEqual(32);
    expect(p.nonce.length).toBeGreaterThanOrEqual(32);
  });

  it("buildAuthorizationUrl includes all required OIDC params", async () => {
    const issuer = "http://localhost:8443";
    const url = buildAuthorizationUrl({
      issuer,
      clientId: "demo",
      redirectUri: "http://localhost:3000/cb",
      scope: "openid profile email offline_access",
      code_challenge: "abc",
      state: "s",
      nonce: "n",
    });
    const u = new URL(url);
    expect(u.origin + u.pathname).toBe(`${issuer}/oauth/v2/authorize`);
    expect(u.searchParams.get("response_type")).toBe("code");
    expect(u.searchParams.get("client_id")).toBe("demo");
    expect(u.searchParams.get("redirect_uri")).toBe("http://localhost:3000/cb");
    expect(u.searchParams.get("scope")).toBe("openid profile email offline_access");
    expect(u.searchParams.get("code_challenge")).toBe("abc");
    expect(u.searchParams.get("code_challenge_method")).toBe("S256");
    expect(u.searchParams.get("state")).toBe("s");
    expect(u.searchParams.get("nonce")).toBe("n");
  });

  it("buildEndSessionUrl includes id_token_hint + post_logout_redirect_uri", () => {
    const u = new URL(
      buildEndSessionUrl({
        issuer: "http://localhost:8443",
        idTokenHint: "abc.def.ghi",
        postLogoutRedirectUri: "http://localhost:3000/",
      }),
    );
    expect(u.pathname).toBe("/oidc/v1/end_session");
    expect(u.searchParams.get("id_token_hint")).toBe("abc.def.ghi");
    expect(u.searchParams.get("post_logout_redirect_uri")).toBe("http://localhost:3000/");
  });
});
