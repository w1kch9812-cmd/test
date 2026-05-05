"use client";

import { Button } from "@gongzzang/ui";
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

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <h2 className="text-2xl font-bold">오류가 발생했어요</h2>
      <p className="text-[var(--color-muted-fg)]">
        잠시 후 다시 시도해 주세요. 문제가 계속되면 관리자에게 문의해 주세요.
      </p>
      <Button onClick={reset}>다시 시도</Button>
    </main>
  );
}
