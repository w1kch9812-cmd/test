### Step 3.3: Wire into `main.rs`

- [ ] **Step 3.3.1: Modify `services/api/src/main.rs` mod declaration**

Edit lines 53-60 (the `mod routes { ... }` block) to add:

```rust
mod routes {
    pub mod admin_listings;
    pub mod auth_event;
    pub mod bookmarks;
    pub mod buildings;       // SP10 T3
    pub mod health;
    pub mod listings;
    pub mod notifications;
    pub mod parcels;         // SP10 T3
}
```

- [ ] **Step 3.3.2: Add state assembly + router merge**

After line 297 (`listings_router` block end), before line 299 (`// SP6-v: 공유 repository`), add:

```rust
    // SP10 T3: Panel system backing endpoints — pure REST resource.
    let parcels_state = routes::parcels::ParcelsState {
        parcel_lookup: listings_state.parcel_lookup.clone(),
    };
    let parcels_router: Router<()> = Router::new()
        .route("/api/parcels/:pnu", get(routes::parcels::get_parcel))
        .with_state(parcels_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // SP10 T3: building_register reader 주입 — SP4-iii-a 의 reader 인스턴스화.
    // 미구현 시 (DATA_GO_KR_API_KEY 미설정) NoOp fallback — 빈 list 반환.
    let building_reader: Arc<dyn routes::buildings::BuildingRegisterReader> =
        Arc::new(NoOpBuildingRegisterReader);
    let buildings_state = routes::buildings::BuildingsState { reader: building_reader };
    let buildings_router: Router<()> = Router::new()
        .route("/api/buildings", get(routes::buildings::list_buildings))
        .with_state(buildings_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));
```

Then in the final `app` builder (line 383-389), add `.merge(parcels_router).merge(buildings_router)`:

```rust
    let app = public
        .merge(protected)
        .merge(listings_router)
        .merge(parcels_router)         // SP10 T3
        .merge(buildings_router)       // SP10 T3
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(internal)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));
```

- [ ] **Step 3.3.3: Add NoOp building reader stub**

At top of `main.rs` (after `use ...` block), add:

```rust
/// SP10 T3: NoOp building reader — DATA_GO_KR_API_KEY 미설정 시 fallback (빈 list).
/// production 은 SP4-iii-a 의 live reader 로 swap.
struct NoOpBuildingRegisterReader;

impl routes::buildings::BuildingRegisterReader for NoOpBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        _pnu: &'a shared_kernel::pnu::Pnu,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<routes::buildings::BuildingItem>>> + Send + 'a>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}
```

- [ ] **Step 3.3.4: Run cargo check**

Run: `cargo check -p api`
Expected: clean.

- [ ] **Step 3.3.5: Run cargo clippy**

Run: `cargo clippy -p api --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3.3.6: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-t3): wire /api/parcels/:pnu + /api/buildings into main router"
```

### Step 3.4: Integration test

- [ ] **Step 3.4.1: Create `services/api/tests/sp10_panel_endpoints.rs`**

```rust
//! SP10 T3: panel backing endpoints integration test.

#[tokio::test]
async fn get_parcel_returns_404_for_unknown_pnu() {
    // ... reuses test scaffolding from existing tests/listing_*.rs
    // — minimum: assert that GET /api/parcels/{19-zeros} with NoOp lookup returns 404
    //   (NoOpParcelInfoLookup returns Ok(None) for any pnu).
    //
    // 실제 test scaffold 는 기존 services/api/tests/*.rs (예: listing_search.rs) 의 setup
    // helper 와 동일 패턴 — Axum app 부팅 + tokio::spawn + reqwest call.
    //
    // 빈 stub 으로 시작 — 첫 fail 후 scaffold 복붙해서 채워나감 (TDD red).
    panic!("write me");
}

#[tokio::test]
async fn get_parcel_returns_400_for_invalid_pnu() {
    panic!("write me");
}

#[tokio::test]
async fn list_buildings_returns_empty_with_noop_reader() {
    panic!("write me");
}
```

- [ ] **Step 3.4.2: Fill scaffold by copying from `services/api/tests/listing_search.rs`**

Read the existing test file pattern and replicate the bootstrap (axum app, port 0, reqwest call). Implement the 3 tests above with concrete asserts. Use `Pnu` constructor for valid 19-digit PNU.

- [ ] **Step 3.4.3: Run integration test**

Run: `cargo test -p api --test sp10_panel_endpoints`
Expected: 3 tests pass.

- [ ] **Step 3.4.4: Commit**

```bash
git add services/api/tests/sp10_panel_endpoints.rs
git commit -m "test(sp10-t3): integration tests for /api/parcels/:pnu + /api/buildings (NoOp path)"
```

---

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

## Task 6: 통합 변경 + lint rules + e2e + a11y + 회귀

**목표:** Spec § 11 통합 변경 적용, § 10.1 lint rules, § 10.2 e2e, § 10.3 extensibility 회귀, § 10.4 텔레메트리 검증.

**Files (modify):**
- `apps/web/app/(authenticated)/listings/page.tsx`
- `apps/web/components/listings/listing-map.tsx`
- `apps/web/components/listings/listing-card.tsx`
- `apps/web/components/listings/listing-card-list.tsx`
- `apps/web/lib/listings/use-listings-query.ts`
- `apps/web/stores/listings.ts`
- `apps/web/lib/listings/filters.ts`
- `apps/web/lib/listings/api.ts` (fetchListings 의 pnu query param 직접화)
- `apps/web/tests/unit/listings/filters.test.ts` (pnu 케이스 제거)
- `apps/web/biome.json`
- `lefthook.yml`

**Files (create):**
- `apps/web/tests/unit/panel-extensibility.test.ts`
- `apps/web/tests/e2e/panel-system.spec.ts`

**Files (delete):**
- `apps/web/components/listings/parcel-info-panel.tsx`

### Step 6.1: Refactor `fetchListings` to accept pnu directly

- [ ] **Step 6.1.1: Modify `apps/web/lib/listings/api.ts`**

Change `FetchListingsInput` and `fetchListings` so that `pnu` is a top-level param, not from filters:

```ts
// Replace the existing FetchListingsInput / fetchListings region with:
export interface FetchListingsInput {
  filters: ListingFilters;
  bounds?: { south: number; west: number; north: number; east: number };
  pnu?: string;
  page?: number;
  size?: number;
}

export async function fetchListings(input: FetchListingsInput): Promise<ListingsResponse> {
  const sp = toSearchParams(input.filters);
  if (input.bounds) {
    const { south, west, north, east } = input.bounds;
    sp.set('bounds', `${south},${west},${north},${east}`);
  }
  if (input.pnu) sp.set('pnu', input.pnu);
  if (input.page !== undefined) sp.set('page', String(input.page));
  if (input.size !== undefined) sp.set('size', String(input.size));

  const json = await api.get(`listings?${sp.toString()}`).json<unknown>();
  return ListingsResponseSchema.parse(json);
}
```

- [ ] **Step 6.1.2: Modify `apps/web/lib/listings/filters.ts`**

Remove `pnu` from `ListingFilters` and from `parseFiltersFromSearchParams` / `toSearchParams`:

```ts
// Edit ListingFilters interface — delete the pnu field:
export interface ListingFilters {
  types: ListingType[];
  transactions: TransactionType[];
  minAreaM2: number | undefined;
  maxAreaM2: number | undefined;
  minPriceKrw: number | undefined;
  maxPriceKrw: number | undefined;
  sort: SortKey;
  adminCode: string | undefined;
  landUseType: string | undefined;
}

// Edit parseFiltersFromSearchParams — remove the pnu line:
//   pnu: sp.get("pnu") ?? undefined,   ← delete

// Edit toSearchParams — remove the pnu branch:
//   if (f.pnu) sp.set("pnu", f.pnu);   ← delete
```

- [ ] **Step 6.1.3: Update `apps/web/tests/unit/listings/filters.test.ts`**

Read the file. Delete any test that checks `pnu` parsing/serialization, since the field is gone.

- [ ] **Step 6.1.4: Run unit tests**

Run: `cd apps/web && pnpm test`
Expected: pass.

- [ ] **Step 6.1.5: Commit**

```bash
git add apps/web/lib/listings/api.ts apps/web/lib/listings/filters.ts apps/web/tests/unit/listings/filters.test.ts
git commit -m "refactor(sp10-t6): fetchListings pnu top-level + remove filters.pnu (panel stack derives)"
```

