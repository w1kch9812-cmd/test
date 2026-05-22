"use client";

import { Button } from "@gongzzang/ui";
import { useTranslations } from "next-intl";
import { useEffect } from "react";

export default function ErrorPage({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);
  const t = useTranslations("errorPage");

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <h2 className="text-2xl font-bold">{t("title")}</h2>
      <p className="text-[var(--color-muted-fg)]">{t("description")}</p>
      <Button onClick={reset}>{t("retry")}</Button>
    </main>
  );
}
