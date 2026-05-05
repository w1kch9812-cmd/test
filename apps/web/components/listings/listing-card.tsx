"use client";
import { Card, CardContent } from "@gongzzang/ui";
import { Heart } from "lucide-react";
import type { Route } from "next";
import Image from "next/image";
import Link from "next/link";
import { useTranslations } from "next-intl";
import type { ListingCard as ListingCardData } from "@/lib/listings/api";
import { formatAreaPyeong, formatPriceKrw } from "@/lib/listings/format";
import { getPinColor } from "@/lib/listings/pin-color";
import { useListingsStore } from "@/stores/listings";

interface ListingCardProps {
  data: ListingCardData;
}

export function ListingCard({ data }: ListingCardProps) {
  const t = useTranslations("listings");
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);
  const isSelected = selectedId === data.id;

  return (
    <Card
      className="overflow-hidden transition-colors"
      style={
        isSelected
          ? { outline: `2px solid var(--color-brand-600)`, outlineOffset: "2px" }
          : { background: undefined }
      }
      onMouseEnter={() => setSelected(data.id)}
      onMouseLeave={() => setSelected(null)}
    >
      <Link href={`/listings/${data.id}` as Route} className="block">
        <div
          className="relative aspect-[4/3] w-full overflow-hidden"
          style={{
            background: data.thumbnail_url ? undefined : `${getPinColor(data.listing_type)}22`,
          }}
        >
          {data.thumbnail_url ? (
            <Image
              src={data.thumbnail_url}
              alt={data.title}
              fill
              className="object-cover"
              sizes="(max-width: 768px) 100vw, 400px"
            />
          ) : (
            <div
              className="flex h-full items-center justify-center text-sm"
              style={{ color: "var(--color-muted-fg)" }}
            >
              {t(`type.${data.listing_type}`)}
            </div>
          )}
        </div>
        <CardContent className="p-4">
          <div className="mb-2 flex items-center gap-2">
            <span
              className="rounded-full px-2 py-0.5 text-xs font-medium text-white"
              style={{ backgroundColor: getPinColor(data.listing_type) }}
            >
              {t(`type.${data.listing_type}`)}
            </span>
            <span className="text-xs" style={{ color: "var(--color-muted-fg)" }}>
              {t(`transaction.${data.transaction_type}`)}
            </span>
          </div>
          <h3 className="mb-1 line-clamp-1 text-base font-semibold">{data.title}</h3>
          <div className="mb-2 text-sm" style={{ color: "var(--color-muted-fg)" }}>
            {formatAreaPyeong(data.area_m2)}
          </div>
          <div className="text-lg font-bold">{formatPriceKrw(data.price_krw)}</div>
          <div
            className="mt-2 flex items-center gap-3 text-xs"
            style={{ color: "var(--color-muted-fg)" }}
          >
            <span title={t("card.viewCount")}>👁 {data.view_count}</span>
            <button
              type="button"
              aria-label={t("card.favoritePlaceholder")}
              className="flex items-center gap-1 transition-colors hover:text-[var(--color-brand-600)]"
              onClick={(e) => {
                e.preventDefault();
                // SP6-iii 가 즐겨찾기 toggle 구현
              }}
            >
              <Heart className="h-3 w-3" /> {data.bookmark_count}
            </button>
          </div>
        </CardContent>
      </Link>
    </Card>
  );
}
