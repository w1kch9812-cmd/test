import { describe, expect, it } from "vitest";
import { ProblemDetails, problem } from "@/lib/http/problem";

describe("ProblemDetails (RFC 7807)", () => {
  it("builds with type, title, status, detail, instance", () => {
    const p = problem({
      type: "auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "보안을 위해 다시 로그인해 주세요.",
      instance: "/api/auth/callback",
    });
    expect(p).toBeInstanceOf(ProblemDetails);
    expect(p.toJSON()).toEqual({
      type: "https://gongzzang.com/errors/auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "보안을 위해 다시 로그인해 주세요.",
      instance: "/api/auth/callback",
    });
  });

  it("toResponse returns content-type application/problem+json", () => {
    const p = problem({ type: "auth/x", title: "t", status: 401 });
    const r = p.toResponse();
    expect(r.status).toBe(401);
    expect(r.headers.get("content-type")).toBe("application/problem+json");
  });
});
