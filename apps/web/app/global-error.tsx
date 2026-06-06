"use client";

import { useEffect } from "react";

import { tStatic } from "@/lib/i18n/static";

export default function GlobalError({
  error,
  unstable_retry,
}: {
  error: Error & { digest?: string };
  unstable_retry: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  const title = tStatic("errorPage.title");

  return (
    <html lang="ko">
      <head>
        <title>{title}</title>
      </head>
      <body>
        <main
          style={{
            alignItems: "center",
            display: "flex",
            flexDirection: "column",
            fontFamily:
              'system-ui, "Segoe UI", Roboto, Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji"',
            gap: "16px",
            justifyContent: "center",
            minHeight: "100vh",
            padding: "32px",
            textAlign: "center",
          }}
        >
          <h2 style={{ fontSize: "24px", fontWeight: 700, margin: 0 }}>{title}</h2>
          <p style={{ color: "#525252", lineHeight: 1.6, margin: 0 }}>
            {tStatic("errorPage.description")}
          </p>
          <button
            onClick={unstable_retry}
            style={{
              background: "#171717",
              border: 0,
              borderRadius: "6px",
              color: "#ffffff",
              cursor: "pointer",
              fontSize: "14px",
              fontWeight: 600,
              height: "36px",
              padding: "0 14px",
            }}
            type="button"
          >
            {tStatic("errorPage.retry")}
          </button>
        </main>
      </body>
    </html>
  );
}
