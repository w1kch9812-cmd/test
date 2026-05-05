import { Button } from "@gongzzang/ui";
import Link from "next/link";

export default function NotFound() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <h2 className="text-2xl font-bold">페이지를 찾을 수 없어요</h2>
      <p className="text-[var(--color-muted-fg)]">주소가 맞는지 다시 한번 확인해 주세요.</p>
      <Button asChild>
        <Link href="/">홈으로 돌아가기</Link>
      </Button>
    </main>
  );
}
