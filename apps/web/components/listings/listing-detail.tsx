"use client";

/**
 * SP6-iii: 매물 상세 client component.
 *
 * 사진 thumbnail grid + 도메인 panel + BookmarkButton. carousel/lightbox 는
 * FU 74. broker contact reveal 은 FU 75.
 */

import { Badge, Card, CardContent, CardHeader, CardTitle } from "@gongzzang/ui";

import { BookmarkButton } from "@/components/listings/bookmark-button";
import type { ListingDetail as ListingDetailData } from "@/lib/listings/api";
import { formatAreaPyeong, formatPriceKrw } from "@/lib/listings/format";

interface ListingDetailProps {
  data: ListingDetailData;
}

const LISTING_TYPE_LABELS: Record<string, string> = {
  factory: "공장",
  warehouse: "창고",
  office: "업무시설",
  knowledge_industry_center: "지식산업센터",
  industrial_land: "산업용지",
  logistics_center: "물류시설",
};

const TRANSACTION_TYPE_LABELS: Record<string, string> = {
  sale: "매매",
  monthly_rent: "월세",
  jeonse: "전세",
};

const STATUS_LABELS: Record<string, string> = {
  draft: "임시저장",
  pending_review: "검토 중",
  active: "공개",
  rejected: "반려",
  sold: "판매 완료",
  expired: "만료",
  archived: "보관됨",
};

export function ListingDetail({ data }: ListingDetailProps): React.ReactElement {
  return (
    <article className="mx-auto max-w-4xl space-y-6 px-4 py-8">
      <header className="flex items-start justify-between gap-4">
        <div className="flex-1">
          <div className="mb-2 flex flex-wrap gap-2">
            <Badge variant="default">
              {LISTING_TYPE_LABELS[data.listing_type] ?? data.listing_type}
            </Badge>
            <Badge variant="default">
              {TRANSACTION_TYPE_LABELS[data.transaction_type] ?? data.transaction_type}
            </Badge>
            {data.status !== "active" && (
              <Badge variant="outline">{STATUS_LABELS[data.status] ?? data.status}</Badge>
            )}
          </div>
          <h1 className="text-2xl font-bold text-[var(--color-ink)]">{data.title}</h1>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            PNU {data.parcel_pnu} · 조회수 {data.view_count}
          </p>
        </div>
        <BookmarkButton
          listingId={data.id}
          isBookmarked={data.is_bookmarked}
          bookmarkCount={data.bookmark_count}
        />
      </header>

      {data.photos.length > 0 ? (
        <section aria-labelledby="photos-heading" className="grid grid-cols-2 gap-3 md:grid-cols-3">
          <h2 id="photos-heading" className="sr-only">
            사진
          </h2>
          {data.photos.map((p) => (
            <PhotoTile key={p.r2_key} photo={p} />
          ))}
        </section>
      ) : (
        <p className="text-sm text-[var(--color-muted)]">사진이 아직 등록되지 않았어요.</p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>거래 정보</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <DataRow label="가격" value={formatPriceKrw(data.price_krw)} />
          {data.deposit_krw != null && (
            <DataRow label="보증금" value={formatPriceKrw(data.deposit_krw)} />
          )}
          {data.monthly_rent_krw != null && (
            <DataRow label="월세" value={formatPriceKrw(data.monthly_rent_krw)} />
          )}
          <DataRow
            label="면적"
            value={`${data.area_m2.toFixed(2)} ㎡ (${formatAreaPyeong(data.area_m2)})`}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>설명</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="whitespace-pre-wrap text-sm text-[var(--color-ink)]">
            {data.description.length > 0 ? data.description : "설명이 없어요."}
          </p>
        </CardContent>
      </Card>
    </article>
  );
}

function DataRow({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <div className="flex justify-between text-sm">
      <span className="text-[var(--color-muted)]">{label}</span>
      <span className="font-medium text-[var(--color-ink)]">{value}</span>
    </div>
  );
}

function PhotoTile({ photo }: { photo: ListingDetailData["photos"][number] }): React.ReactElement {
  // SP4-iii-e R2 통합 후 r2_key 가 실 URL 로 매핑. 1차는 placeholder
  // (R2 base URL 부재 시 alt 만 표시).
  return (
    <div className="aspect-square overflow-hidden rounded-md bg-[var(--color-surface-cream-strong)]">
      <span className="flex h-full items-center justify-center text-xs text-[var(--color-muted)]">
        {photo.caption ?? photo.r2_key}
      </span>
    </div>
  );
}
