/**
 * SP6-iv: `/listings/new` — broker 매물 등록 페이지.
 *
 * `proxy.ts` 가 진입 시 `Broker` (또는 `Admin`) 역할 강제. middleware 통과 후
 * 본 페이지가 폼만 렌더 — 실 검증은 server-side 도메인.
 */

import { ListingForm } from "@/components/listings/listing-form";

export default function NewListingPage(): React.ReactElement {
  return (
    <main className="mx-auto max-w-3xl px-4 py-10">
      <header className="mb-6">
        <h1 className="text-2xl font-bold text-[var(--color-ink)]">매물 등록</h1>
        <p className="mt-2 text-sm text-[var(--color-muted)]">
          매물 정보를 정확하게 입력해 주세요. 등록 후 검토 요청을 보내면 관리자가 확인해요.
        </p>
      </header>
      <ListingForm />
    </main>
  );
}
