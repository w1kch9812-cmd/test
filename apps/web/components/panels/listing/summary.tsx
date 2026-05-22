"use client";
import { Badge } from "@gongzzang/ui";
import { MEDIA_QUERIES_MAX } from "@gongzzang/ui/tokens.js";
import Image from "next/image";
import { useTranslations } from "next-intl";
import { useState } from "react";
import type { ListingDetail } from "@/lib/listings/api";
import { formatAreaPyeong, formatPriceKrw } from "@/lib/listings/format";
import { listingPhotoImageSrc } from "@/lib/listings/photos";
import type { PanelStackEntry } from "@/lib/panel/types";

export function ListingSummaryCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: "listing" }>;
  data: ListingDetail;
}) {
  const t = useTranslations("panels.listing.summary");
  const [imageBroken, setImageBroken] = useState(false);
  const cover = data.photos?.[0];

  return (
    <div className="flex flex-col gap-4 p-6">
      {cover && !imageBroken && (
        <div className="relative aspect-[4/3] w-full overflow-hidden rounded-md bg-[var(--color-surface-cream-strong)]">
          <Image
            src={listingPhotoImageSrc(entry.id, cover)}
            alt={data.title}
            fill
            className="object-cover"
            sizes={`${MEDIA_QUERIES_MAX.xl} 100vw, 600px`}
            onError={() => setImageBroken(true)}
          />
        </div>
      )}
      {cover && imageBroken && (
        <div className="aspect-[4/3] w-full overflow-hidden rounded-md bg-[var(--color-surface-cream-strong)] p-4 text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {cover.caption ?? t("photoUnavailable")}
        </div>
      )}
      <header className="flex items-center gap-2">
        <Badge>{t(`type.${data.listing_type}` as never)}</Badge>
        <Badge variant="outline">{t(`transaction.${data.transaction_type}` as never)}</Badge>
      </header>
      <h2 className="text-[length:var(--text-title-lg)] font-semibold text-[var(--color-ink)]">
        {data.title}
      </h2>
      <dl className="grid grid-cols-2 gap-y-2 text-[length:var(--text-body-sm)]">
        <dt className="text-[var(--color-muted)]">{t("area")}</dt>
        <dd>{formatAreaPyeong(data.area_m2)}</dd>
        <dt className="text-[var(--color-muted)]">{t("price")}</dt>
        <dd>{formatPriceKrw(data.price_krw)}</dd>
        <dt className="text-[var(--color-muted)]">PNU</dt>
        <dd className="font-mono">{data.parcel_pnu}</dd>
      </dl>
      <p className="whitespace-pre-wrap text-[length:var(--text-body-sm)] text-[var(--color-muted)]">
        {data.description}
      </p>
    </div>
  );
}
