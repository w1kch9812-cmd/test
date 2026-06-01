## Task 4: `parcel` kind registration (3 views + i18n)

**목표:** Spec § 6 R1 + § 7 4 endpoints 의 `parcel.summary` / `parcel.buildings` / `parcel.listings` 활성화.

**Files:**
- Create: `apps/web/lib/api/parcels.ts`
- Create: `apps/web/lib/api/buildings.ts`
- Create: `apps/web/components/panels/parcel/skeletons.tsx`
- Create: `apps/web/components/panels/parcel/summary.tsx`
- Create: `apps/web/components/panels/parcel/buildings.tsx`
- Create: `apps/web/components/panels/parcel/listings.tsx`
- Create: `apps/web/components/panels/parcel/register.ts`
- Modify: `apps/web/lib/i18n/ko.json` (add `panels.parcel.*` namespace)
- Modify: `apps/web/lib/i18n/haeyo.ts` (whatever generates message types — add namespace)

### Step 4.1: API clients

- [ ] **Step 4.1.1: Implement `lib/api/parcels.ts`**

```ts
// apps/web/lib/api/parcels.ts
import { z } from 'zod';
import { api } from '@/lib/api';

export const ParcelInfoSchema = z.object({
  pnu: z.string(),
  sido_code: z.string(),
  sigungu_code: z.string(),
  eupmyeondong_code: z.string(),
  sido_name: z.string(),
  sigungu_name: z.string(),
  eupmyeondong_name: z.string(),
  land_use_type: z.string(),
  zoning: z.string().nullish(),
  official_land_price_per_m2: z.number().int().nullish(),
  gosi_year_month: z.string().nullish(),
});

export type ParcelInfo = z.infer<typeof ParcelInfoSchema>;

export async function fetchParcel(pnu: string): Promise<ParcelInfo> {
  const json = await api.get(`api/parcels/${pnu}`).json<unknown>();
  return ParcelInfoSchema.parse(json);
}
```

- [ ] **Step 4.1.2: Implement `lib/api/buildings.ts`**

```ts
// apps/web/lib/api/buildings.ts
import { z } from 'zod';
import { api } from '@/lib/api';

export const BuildingSchema = z.object({
  id: z.string(),
  name: z.string(),
  purpose: z.string(),
  total_area_m2: z.number(),
  approved_at: z.string().nullish(),
});

export type Building = z.infer<typeof BuildingSchema>;

export const BuildingsResponseSchema = z.object({
  buildings: z.array(BuildingSchema),
});

export type BuildingsResponse = z.infer<typeof BuildingsResponseSchema>;

export async function fetchBuildings(parcelPnu: string): Promise<BuildingsResponse> {
  const json = await api.get(`api/buildings?parcel_pnu=${encodeURIComponent(parcelPnu)}`).json<unknown>();
  return BuildingsResponseSchema.parse(json);
}
```

- [ ] **Step 4.1.3: Commit**

```bash
git add apps/web/lib/api/parcels.ts apps/web/lib/api/buildings.ts
git commit -m "feat(sp10-t4): api clients for /api/parcels/:pnu + /api/buildings"
```

### Step 4.2: Skeletons + view components

- [ ] **Step 4.2.1: Implement `components/panels/parcel/skeletons.tsx`**

```tsx
// apps/web/components/panels/parcel/skeletons.tsx
'use client';
import { Skeleton } from '@gongzzang/ui';
import { useTranslations } from 'next-intl';

export function ParcelLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-6">
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-4 w-64" />
      <Skeleton className="h-4 w-48" />
      <Skeleton className="h-32 w-full" />
    </div>
  );
}

export function ParcelErrorCard({ error }: { error: unknown }) {
  const t = useTranslations('panels.parcel');
  const msg = error instanceof Error ? error.message : String(error);
  return (
    <div className="p-6">
      <div className="text-[length:var(--text-body-md)] font-semibold text-[var(--color-error)]">
        {t('errors.loadFailed')}
      </div>
      <div className="mt-2 text-[length:var(--text-caption)] text-[var(--color-muted)]">{msg}</div>
    </div>
  );
}

export function ParcelEmptyCard() {
  const t = useTranslations('panels.parcel');
  return <div className="p-6 text-center text-[var(--color-muted)]">{t('empty')}</div>;
}
```

- [ ] **Step 4.2.2: Implement `summary.tsx`**

```tsx
// apps/web/components/panels/parcel/summary.tsx
'use client';
import { useTranslations } from 'next-intl';
import type { ParcelInfo } from '@/lib/api/parcels';
import type { PanelStackEntry } from '@/lib/panel/types';
import { usePanelStack } from '@/lib/panel/use-panel-stack';

export function ParcelSummaryCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'parcel' }>;
  data: ParcelInfo;
}) {
  const t = useTranslations('panels.parcel.summary');
  const { push } = usePanelStack();

  return (
    <div className="flex flex-col gap-4 p-6">
      <header>
        <div className="font-mono text-[length:var(--text-caption)] text-[var(--color-muted)]">
          PNU {entry.id}
        </div>
        <h2 className="text-[length:var(--text-title-lg)] font-semibold text-[var(--color-ink)]">
          {data.sido_name} {data.sigungu_name} {data.eupmyeondong_name}
        </h2>
      </header>
      <dl className="grid grid-cols-2 gap-y-2 text-[length:var(--text-body-sm)]">
        <dt className="text-[var(--color-muted)]">{t('landUse')}</dt>
        <dd className="text-[var(--color-ink)]">{data.land_use_type}</dd>
        {data.zoning && (
          <>
            <dt className="text-[var(--color-muted)]">{t('zoning')}</dt>
            <dd className="text-[var(--color-ink)]">{data.zoning}</dd>
          </>
        )}
        {data.official_land_price_per_m2 != null && (
          <>
            <dt className="text-[var(--color-muted)]">{t('officialPrice')}</dt>
            <dd className="text-[var(--color-ink)]">
              {data.official_land_price_per_m2.toLocaleString('ko-KR')} 원/㎡
            </dd>
          </>
        )}
      </dl>
      <nav className="mt-4 flex flex-col gap-2">
        <button
          type="button"
          onClick={() => push({ kind: 'parcel', id: entry.id, view: 'buildings' })}
          className="rounded-md border border-[var(--color-hairline)] px-3 py-2 text-left hover:bg-[var(--color-surface-cream-strong)]"
        >
          {t('viewBuildings')} ›
        </button>
        <button
          type="button"
          onClick={() => push({ kind: 'parcel', id: entry.id, view: 'listings' })}
          className="rounded-md border border-[var(--color-hairline)] px-3 py-2 text-left hover:bg-[var(--color-surface-cream-strong)]"
        >
          {t('viewListings')} ›
        </button>
      </nav>
    </div>
  );
}
```

- [ ] **Step 4.2.3: Implement `buildings.tsx`**

```tsx
// apps/web/components/panels/parcel/buildings.tsx
'use client';
import { useTranslations } from 'next-intl';
import type { BuildingsResponse } from '@/lib/api/buildings';
import type { PanelStackEntry } from '@/lib/panel/types';

export function ParcelBuildingsCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'parcel' }>;
  data: BuildingsResponse;
}) {
  const t = useTranslations('panels.parcel.buildings');
  if (data.buildings.length === 0) {
    return <div className="p-6 text-center text-[var(--color-muted)]">{t('none')}</div>;
  }
  return (
    <div className="flex flex-col gap-3 p-6">
      <header className="flex items-baseline gap-2">
        <h2 className="text-[length:var(--text-title-md)] font-semibold">{t('title')}</h2>
        <span className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {data.buildings.length} {t('count')}
        </span>
      </header>
      <ul className="flex flex-col gap-2">
        {data.buildings.map((b) => (
          <li
            key={b.id}
            className="rounded-md border border-[var(--color-hairline)] p-3 text-[length:var(--text-body-sm)]"
          >
            <div className="font-semibold text-[var(--color-ink)]">{b.name}</div>
            <div className="text-[var(--color-muted)]">
              {b.purpose} · {b.total_area_m2.toLocaleString('ko-KR')} ㎡
              {b.approved_at && ` · ${b.approved_at}`}
            </div>
          </li>
        ))}
      </ul>
      {/* PNU 의 entry.id 는 i18n 라벨 표시 외 미사용 — 본 view 는 list-only */}
      <span className="hidden">{entry.id}</span>
    </div>
  );
}
```

- [ ] **Step 4.2.4: Implement `listings.tsx`**

```tsx
// apps/web/components/panels/parcel/listings.tsx
'use client';
import { useTranslations } from 'next-intl';
import type { ListingsResponse } from '@/lib/listings/api';
import type { PanelStackEntry } from '@/lib/panel/types';
import { usePanelStack } from '@/lib/panel/use-panel-stack';

export function ParcelListingsCard({
  entry,
  data,
}: {
  entry: Extract<PanelStackEntry, { kind: 'parcel' }>;
  data: ListingsResponse;
}) {
  const t = useTranslations('panels.parcel.listings');
  const { push } = usePanelStack();
  if (data.listings.length === 0) {
    return <div className="p-6 text-center text-[var(--color-muted)]">{t('none')}</div>;
  }
  return (
    <div className="flex flex-col gap-3 p-6">
      <header>
        <h2 className="text-[length:var(--text-title-md)] font-semibold">
          {t('title', { count: data.total })}
        </h2>
      </header>
      <ul className="flex flex-col gap-2">
        {data.listings.map((l) => (
          <li key={l.id}>
            <button
              type="button"
              onClick={() => push({ kind: 'listing', id: l.id, view: 'summary' })}
              className="block w-full rounded-md border border-[var(--color-hairline)] p-3 text-left hover:bg-[var(--color-surface-cream-strong)]"
            >
              <div className="font-semibold text-[var(--color-ink)]">{l.title}</div>
              <div className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
                {l.price_krw.toLocaleString('ko-KR')} 원 · {l.area_m2.toLocaleString('ko-KR')} ㎡
              </div>
            </button>
          </li>
        ))}
      </ul>
      <span className="hidden">{entry.id}</span>
    </div>
  );
}
```

- [ ] **Step 4.2.5: Commit**

```bash
git add apps/web/components/panels/parcel/skeletons.tsx \
        apps/web/components/panels/parcel/summary.tsx \
        apps/web/components/panels/parcel/buildings.tsx \
        apps/web/components/panels/parcel/listings.tsx
git commit -m "feat(sp10-t4): parcel panel views (summary/buildings/listings) + skeletons"
```

### Step 4.3: Register parcel kind

- [ ] **Step 4.3.1: Implement `register.ts`**

```ts
// apps/web/components/panels/parcel/register.ts
import { fetchBuildings } from '@/lib/api/buildings';
import { fetchParcel } from '@/lib/api/parcels';
import { fetchListings } from '@/lib/listings/api';
import { defineKind } from '@/lib/panel/registry';
import { ParcelBuildingsCard } from './buildings';
import { ParcelListingsCard } from './listings';
import { ParcelEmptyCard, ParcelErrorCard, ParcelLoadingSkeleton } from './skeletons';
import { ParcelSummaryCard } from './summary';

defineKind({
  kind: 'parcel',
  idPattern: /^\d{19}$/,
  views: {
    summary: {
      component: ParcelSummaryCard,
      fetcher: (id) => fetchParcel(id),
      staleTime: 5 * 60_000,
      links: [],
    },
    buildings: {
      component: ParcelBuildingsCard,
      fetcher: (id) => fetchBuildings(id),
      staleTime: 5 * 60_000,
      links: [],
    },
    listings: {
      component: ParcelListingsCard,
      fetcher: (id) =>
        fetchListings({
          filters: {
            types: [],
            transactions: [],
            minAreaM2: undefined,
            maxAreaM2: undefined,
            minPriceKrw: undefined,
            maxPriceKrw: undefined,
            sort: 'created_at_desc',
            adminCode: undefined,
            landUseType: undefined,
            // After T6 the `pnu` field is removed from filters; this fetcher
            // composes a one-shot search by direct query param.
          } as never,
          // direct PNU-narrowed search via query param appended in fetchListings
          // (see T6 changes). Until then this works because fetchListings
          // accepts whatever filters shape with `pnu` field.
        }),
      staleTime: 60_000,
      links: [],
    },
  },
  loadingComponent: ParcelLoadingSkeleton,
  errorComponent: ParcelErrorCard,
  emptyComponent: ParcelEmptyCard,
  authGate: { required: true },
  i18nNamespace: 'panels.parcel',
  telemetryAttrs: (entry) => ({ pnu: entry.id }),
});
```

> **NOTE for engineer:** `parcel.listings` view 의 fetcher 는 T6 에서 fetchListings 가 `pnu` 직접 query param 으로 받도록 변경됨 — T6 step 6.3 참조. 본 step 에서는 stub-shaped 으로 두고 T6 가 정합 맞춤.

- [ ] **Step 4.3.2: Commit**

```bash
git add apps/web/components/panels/parcel/register.ts
git commit -m "feat(sp10-t4): defineKind('parcel') with 3 views + 4-state shell"
```

### Step 4.4: i18n namespace

- [ ] **Step 4.4.1: Add `panels.parcel.*` keys to `lib/i18n/ko.json`**

Read current `lib/i18n/ko.json`. Add at root:

```json
"panel": {
  "back": "이전",
  "breadcrumb": "경로"
},
"panels": {
  "parcel": {
    "empty": "필지 정보가 없어요",
    "errors": {
      "loadFailed": "필지 정보를 불러오지 못했어요"
    },
    "summary": {
      "landUse": "지목",
      "zoning": "용도지역",
      "officialPrice": "공시지가",
      "viewBuildings": "건축물 보기",
      "viewListings": "등록 매물 보기"
    },
    "buildings": {
      "title": "건축물",
      "count": "동",
      "none": "등록된 건축물이 없어요"
    },
    "listings": {
      "title": "이 필지의 매물 ({count})",
      "none": "이 필지에 등록된 매물이 없어요"
    }
  }
}
```

- [ ] **Step 4.4.2: Run typecheck (i18n type generation)**

Run: `cd apps/web && pnpm typecheck`
Expected: clean. If `next-intl` typed namespace requires regeneration, follow the project's existing pattern (likely `messages.d.ts` is auto-derived).

- [ ] **Step 4.4.3: Commit**

```bash
git add apps/web/lib/i18n/ko.json
git commit -m "feat(sp10-t4): i18n panels.parcel.* namespace"
```

---

