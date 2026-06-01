## Task 5: `listing` kind registration + redirect

**Files:**
- Create: `apps/web/components/panels/listing/skeletons.tsx`
- Create: `apps/web/components/panels/listing/summary.tsx`
- Create: `apps/web/components/panels/listing/register.ts`
- Modify: `apps/web/app/(authenticated)/listings/[id]/page.tsx` (server redirect)
- Modify: `apps/web/lib/i18n/ko.json` (add `panels.listing.*`)

### Step 5.1: Listing summary view

- [ ] **Step 5.1.1: Implement `skeletons.tsx`**

```tsx
// apps/web/components/panels/listing/skeletons.tsx
'use client';
import { Skeleton } from '@gongzzang/ui';
import { useTranslations } from 'next-intl';

export function ListingLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-6">
      <Skeleton className="aspect-[4/3] w-full" />
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-4 w-64" />
    </div>
  );
}

export function ListingErrorCard({ error }: { error: unknown }) {
  const t = useTranslations('panels.listing');
  return (
    <div className="p-6">
      <div className="text-[var(--color-error)]">{t('errors.loadFailed')}</div>
      <div className="mt-2 text-[length:var(--text-caption)] text-[var(--color-muted)]">
        {error instanceof Error ? error.message : String(error)}
      </div>
    </div>
  );
}

export function ListingEmptyCard() {
  const t = useTranslations('panels.listing');
  return <div className="p-6 text-center text-[var(--color-muted)]">{t('notFound')}</div>;
}
```

- [ ] **Step 5.1.2: Implement `summary.tsx`**

```tsx
// apps/web/components/panels/listing/summary.tsx
'use client';
import { Badge } from '@gongzzang/ui';
import Image from 'next/image';
import { useTranslations } from 'next-intl';
import type { ListingDetail } from '@/lib/listings/api';
import { formatAreaPyeong, formatPriceKrw } from '@/lib/listings/format';
import type { PanelStackEntry } from '@/lib/panel/types';

export function ListingSummaryCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'listing' }>;
  data: ListingDetail;
}) {
  const t = useTranslations('panels.listing.summary');
  const cover = data.photos?.[0];

  return (
    <div className="flex flex-col gap-4 p-6">
      {cover && (
        <div className="relative aspect-[4/3] w-full overflow-hidden rounded-md bg-[var(--color-surface-cream-strong)]">
          <Image
            src={`/api/listings/${entry.id}/photos/${cover.r2_key}`}
            alt={data.title}
            fill
            className="object-cover"
            sizes="(max-width: 1280px) 100vw, 600px"
          />
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
        <dt className="text-[var(--color-muted)]">{t('area')}</dt>
        <dd>{formatAreaPyeong(data.area_m2)}</dd>
        <dt className="text-[var(--color-muted)]">{t('price')}</dt>
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
```

- [ ] **Step 5.1.3: Implement `register.ts`**

```ts
// apps/web/components/panels/listing/register.ts
import { fetchListingDetail } from '@/lib/listings/api';
import { defineKind } from '@/lib/panel/registry';
import { ListingEmptyCard, ListingErrorCard, ListingLoadingSkeleton } from './skeletons';
import { ListingSummaryCard } from './summary';

defineKind({
  kind: 'listing',
  idPattern: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
  views: {
    summary: {
      component: ListingSummaryCard,
      fetcher: (id) => fetchListingDetail(id),
      staleTime: 60_000,
      links: [],
    },
  },
  loadingComponent: ListingLoadingSkeleton,
  errorComponent: ListingErrorCard,
  emptyComponent: ListingEmptyCard,
  authGate: { required: true },
  i18nNamespace: 'panels.listing',
  telemetryAttrs: (entry) => ({ listing_id: entry.id }),
});
```

- [ ] **Step 5.1.4: Commit**

```bash
git add apps/web/components/panels/listing/
git commit -m "feat(sp10-t5): defineKind('listing') with summary view + 4-state shell"
```

### Step 5.2: Server redirect for `/listings/[id]`

- [ ] **Step 5.2.1: Replace `apps/web/app/(authenticated)/listings/[id]/page.tsx` body**

```tsx
// apps/web/app/(authenticated)/listings/[id]/page.tsx
/**
 * SP10: 매물 상세 page → /listings?p=listing:{id}.summary 로 server redirect.
 * 컴포넌트 사본 0 (spec rule § 9 #13). Middle-click / new-tab 도 redirect 가 받음.
 */
import { redirect } from 'next/navigation';

interface PageProps {
  params: Promise<{ id: string }>;
}

export default async function ListingDetailPage({ params }: PageProps): Promise<never> {
  const { id } = await params;
  redirect(`/listings?p=listing:${encodeURIComponent(id)}.summary`);
}
```

- [ ] **Step 5.2.2: Commit**

```bash
git add apps/web/app/(authenticated)/listings/[id]/page.tsx
git commit -m "feat(sp10-t5): /listings/[id] server redirect → /listings?p=listing:id.summary (sample 0)"
```

### Step 5.3: Listing i18n keys

- [ ] **Step 5.3.1: Add `panels.listing.*` to `lib/i18n/ko.json`**

Append (inside the existing `panels` object from T4):

```json
"listing": {
  "notFound": "매물을 찾을 수 없어요",
  "errors": {
    "loadFailed": "매물 정보를 불러오지 못했어요"
  },
  "summary": {
    "area": "면적",
    "price": "가격",
    "type": {
      "factory": "공장",
      "warehouse": "창고",
      "office": "오피스",
      "knowledge_industry_center": "지식산업센터",
      "industrial_land": "산업용지",
      "logistics_center": "물류센터"
    },
    "transaction": {
      "sale": "매매",
      "monthly_rent": "월세",
      "jeonse": "전세"
    }
  }
}
```

- [ ] **Step 5.3.2: Commit**

```bash
git add apps/web/lib/i18n/ko.json
git commit -m "feat(sp10-t5): i18n panels.listing.* namespace"
```

---

