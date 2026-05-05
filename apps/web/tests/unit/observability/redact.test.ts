import pino from "pino";
import { describe, expect, it } from "vitest";
import { REDACT_PATHS } from "@/lib/observability/redact";

describe("pino redaction", () => {
  it("masks access_token / refresh_token / ci / password", async () => {
    const lines: string[] = [];
    const logger = pino(
      { redact: { paths: REDACT_PATHS, censor: "[REDACTED]" } },
      {
        write: (s) => {
          lines.push(s);
        },
      },
    );
    logger.info(
      {
        access_token: "secret1",
        refresh_token: "secret2",
        ci: "K7H2-CI-VALUE",
        password: "p",
        normal: "ok",
      },
      "test",
    );
    const log = lines.join("\n");
    expect(log).toContain("[REDACTED]");
    expect(log).not.toContain("secret1");
    expect(log).not.toContain("secret2");
    expect(log).not.toContain("K7H2-CI-VALUE");
    expect(log).toContain("normal");
    expect(log).toContain("ok");
  });
});
