"use client";
import { Badge, Card } from "@gongzzang/ui";
import { Heart } from "lucide-react";
import type { Route } from "next";
import Image from "next/image";
import Link from "next/link";
import { useTranslations } from "next-intl";
import type { ListingCard as ListingCardData } from "@/lib/listings/api";
import { formatAreaPyeong, formatPriceKrw } from "@/lib/listings/format";
import { usePanelStack } from "@/lib/panel/use-panel-stack";

/*
 * ListingCard — Claude.com spec 의 feature-card 패턴 (cream-card surface).
 * 핀↔카드 highlight: 선택 시 coral 외곽선 (spec 의 primary 강조 사용).
 * 카드 내부 구조: 이미지(4/3) + 타입/거래방식 badge + 제목 + 면적 + 가격 + 메타.
 *
 * SP10: Click → pushPanel({ kind: "listing", view: "summary" }).
 * Cmd/Ctrl-click 은 새 탭으로 그대로 흘려보내고 server redirect 가 받음.
 * 선택 상태는 panel stack top 에서 derive — store 의 selectedListingId 폐기됨.
 */
interface ListingCardProps {
  data: ListingCardData;
}

export function ListingCard({ data }: ListingCardProps) {
  const t = useTranslations("listings");
  const { push, stack } = usePanelStack();
  const top = stack.entries.at(-1);
  const isSelected = top?.kind === "listing" && top.id === data.id;

  return (
    <Card
      surface="cream-card"
      className={[
        "overflow-hidden transition-shadow",
        isSelected
          ? "ring-2 ring-[var(--color-primary)] ring-offset-2 ring-offset-[var(--color-canvas)]"
          : "",
      ].join(" ")}
    >
      <Link
        href={`/listings/${data.id}` as Route}
        onClick={(e) => {
          // Cmd/Ctrl-click / middle-click 은 그대로 새 탭 — server redirect 가 받음.
          if (e.metaKey || e.ctrlKey || e.button === 1) return;
          e.preventDefault();
          push({ kind: "listing", id: data.id, view: "summary" });
        }}
        className="block"
      >
        <div className="relative aspect-[4/3] w-full overflow-hidden bg-[var(--color-surface-cream-strong)]">
          {data.thumbnail_url ? (
            <Image
              src={data.thumbnail_url}
              alt={data.title}
              fill
              className="object-cover"
              sizes="(max-width: 768px) 100vw, 420px"
            />
          ) : (
            <div className="flex h-full items-center justify-center text-[length:var(--text-caption)] text-[var(--color-muted)]">
              {t(`type.${data.listing_type}`)}
            </div>
          )}
        </div>
        <div className="flex flex-col gap-2 p-5">
          <div className="flex items-center gap-2">
            <Badge variant="default">{t(`type.${data.listing_type}`)}</Badge>
            <Badge variant="outline">{t(`transaction.${data.transaction_type}`)}</Badge>
          </div>
          <h3 className="line-clamp-1 text-[length:var(--text-title-md)] font-semibold leading-[var(--leading-title)] text-[var(--color-ink)]">
            {data.title}
          </h3>
          <div className="text-[length:var(--text-body-sm)] text-[var(--color-muted)]">
            {formatAreaPyeong(data.area_m2)}
          </div>
          <div className="text-[length:var(--text-title-lg)] font-semibold tracking-[var(--tracking-display-sm)] text-[var(--color-ink)]">
            {formatPriceKrw(data.price_krw)}
          </div>
          <div className="mt-1 flex items-center gap-4 text-[length:var(--text-caption)] text-[var(--color-muted)]">
            <span title={t("card.viewCount")}>조회 {data.view_count}</span>
            <button
              type="button"
              aria-label={t("card.favoritePlaceholder")}
              className="inline-flex items-center gap-1 transition-colors hover:text-[var(--color-primary)]"
              onClick={(e) => {
                e.preventDefault();
                // SP6-iii 가 즐겨찾기 toggle 구현
              }}
            >
              <Heart className="h-3.5 w-3.5" /> {data.bookmark_count}
            </button>
          </div>
        </div>
      </Link>
    </Card>
  );
}
