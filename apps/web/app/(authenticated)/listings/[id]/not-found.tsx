import type { Route } from "next";
import Link from "next/link";

export default function NotFound(): React.ReactElement {
  return (
    <main className="mx-auto max-w-md px-4 py-20 text-center">
      <h1 className="text-2xl font-bold text-[var(--color-ink)]">매물을 찾을 수 없어요</h1>
      <p className="mt-2 text-sm text-[var(--color-muted)]">
        삭제된 매물이거나 권한이 없는 매물일 수 있어요.
      </p>
      <Link
        href={"/listings" as Route}
        className="mt-6 inline-block text-sm text-[var(--color-primary)] underline"
      >
        매물 검색으로 돌아가기
      </Link>
    </main>
  );
}
