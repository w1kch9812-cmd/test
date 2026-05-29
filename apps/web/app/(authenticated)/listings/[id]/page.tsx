/**
 * SP10: 매물 상세 page → /listings?p=listing:{id}.summary 로 server redirect.
 * 컴포넌트 사본 0 (spec rule § 9 #13). Middle-click / new-tab 도 redirect 가 받음.
 */
import { redirect } from "next/navigation";
import { ROUTES } from "@/lib/routes";

interface PageProps {
  params: Promise<{ id: string }>;
}

export default async function ListingDetailPage({ params }: PageProps): Promise<never> {
  const { id } = await params;
  redirect(`${ROUTES.listings.index}?p=listing:${encodeURIComponent(id)}.summary`);
}
