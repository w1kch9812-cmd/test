"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@gongzzang/ui";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export default function Home() {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["healthz"],
    queryFn: () => api.get("healthz").text(),
  });

  return (
    <main className="container mx-auto flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>공짱 Foundation Smoke</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <p className="text-sm text-[var(--color-muted-fg)]">
            /api/proxy/healthz → backend /healthz 호출 확인.
          </p>
          {isLoading && <p>불러오는 중이에요…</p>}
          {error && (
            <p className="text-[var(--color-destructive)]" role="alert">
              호출 실패: {error.message}
            </p>
          )}
          {data && (
            <p className="font-mono text-sm" data-testid="healthz-response">
              응답: {data}
            </p>
          )}
          <Button onClick={() => refetch()} variant="outline">
            다시 호출
          </Button>
        </CardContent>
      </Card>
    </main>
  );
}
