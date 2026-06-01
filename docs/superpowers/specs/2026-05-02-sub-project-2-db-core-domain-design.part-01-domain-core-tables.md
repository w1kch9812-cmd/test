# Sub-project 2 DB Core Domain Design - Part 01: Domain Classification And Core Tables

Parent index: [Sub-project 2 DB Core Domain Design](./2026-05-02-sub-project-2-db-core-domain-design.md).

## 4. 도메인 분류 — R2 정적 vs RDS 동적

### 원칙
- **외부 공공 데이터** = R2 정적 (V-World/data.go.kr/법제처 응답 → PMTiles/JSON)
- **사용자 등록 콘텐츠** = RDS 동적 (매물, 사용자 활동)
- **사용자 활동** = RDS 동적 (북마크, 검색, 분석 리포트)
- **바이너리 파일** = R2 (사진, presigned URL 업로드)

### 도메인 BC 매핑

```
Core BC:
  RDS 동적 ──── User
              Listing
  R2 정적 ───── Parcel
              Building
              IndustrialComplex
              Manufacturer
  공유 ───── shared-kernel (Pnu, Money, Area, BusinessNumber 등)

Market BC:
  R2 정적 ───── RealTransaction (이력)
              CourtAuction (활성+이력)
  RDS 동적 ──── Subscription (Phase 2+)
              Inquiry (Phase 2+)

Regulation BC:
  R2 정적 ───── Law
              Regulation

Insights BC:
  RDS 동적 ──── Bookmark (listing은 FK, 나머지는 polymorphic)
              SearchHistory
              AnalysisReport
              Notification

Admin/Operations:
  RDS 동적 ──── AdminAction
              BusinessVerificationQueue
              ListingReviewQueue
              ListingReport
              FeaturedContent
              SystemAlert

System:
  RDS 동적 ──── AuditLog (1년 + R2 archive)
              OutboxEvent
              PipelineSchedule
              PipelineRun (steps JSONB)
```

---

## 5. RDS 테이블 18개 (V001__init.sql)

### 5.1 사용자 + 매물 (Core BC 동적)

#### `user`

```sql
create table "user" (
    id char(30) primary key,                            -- usr_01HXY...
    zitadel_sub varchar(255) not null unique,           -- Zitadel JWT sub claim
    email varchar(320) not null unique,
    phone_kr_hash varchar(64),                          -- SHA-256 해시 (PIPA)
    display_name varchar(100) not null,
    user_kind varchar(20) not null check (user_kind in ('individual', 'corporation')),
    business_number varchar(12),                        -- XXX-XX-XXXXX (검증된 사업자만)
    business_verified_at timestamptz,
    broker_license_number varchar(50),                  -- 공인중개사 자격번호
    broker_verified_at timestamptz,
    roles text[] not null default '{}',                 -- ['Buyer','Seller','Broker','Developer','Enterprise','Operator','Admin']
    nice_verified_at timestamptz,                       -- NICE 본인인증
    marketing_consent_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    last_login_at timestamptz,
    deleted_at timestamptz,                             -- Soft delete (PIPA RTBF)
    version bigint not null default 1
);

create index user_business_number_idx on "user"(business_number) where business_number is not null;
create index user_roles_idx on "user" using gin(roles);
create index user_active_idx on "user"(created_at desc) where deleted_at is null;
```

#### `listing`

```sql
create table listing (
    id char(30) primary key,                            -- lst_...
    owner_id char(30) not null references "user"(id),
    parcel_pnu char(19) not null,                       -- R2의 Parcel과 매핑 (FK 아님 — R2이라)
    listing_type varchar(30) not null check (listing_type in
        ('factory', 'warehouse', 'office', 'knowledge_industry_center', 'industrial_land', 'logistics_center')),
    transaction_type varchar(20) not null check (transaction_type in
        ('sale', 'monthly_rent', 'jeonse')),
    price_krw bigint not null check (price_krw > 0),
    deposit_krw bigint check (deposit_krw is null or deposit_krw >= 0),
    monthly_rent_krw bigint check (monthly_rent_krw is null or monthly_rent_krw >= 0),
    area_m2 numeric(12, 2) not null check (area_m2 > 0),
    title varchar(200) not null,
    description text not null default '',
    status varchar(20) not null default 'draft' check (status in
        ('draft', 'pending_review', 'active', 'sold', 'expired', 'rejected')),
    contact_visibility varchar(20) not null default 'login_required' check
        (contact_visibility in ('public', 'login_required', 'verified_only')),
    view_count bigint not null default 0,
    bookmark_count bigint not null default 0,
    geom_point geometry(Point, 4326),                   -- 매물 위치 (지도 마커)
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    expires_at timestamptz,
    version bigint not null default 1,
    constraint listing_transaction_fields_chk check (
        (transaction_type = 'sale' and deposit_krw is null and monthly_rent_krw is null)
        or (transaction_type = 'monthly_rent' and deposit_krw is not null and monthly_rent_krw is not null)
        or (transaction_type = 'jeonse' and deposit_krw is not null and monthly_rent_krw is null)
    )
);

create index listing_status_idx on listing(status);
create index listing_listing_type_idx on listing(listing_type);
create index listing_owner_idx on listing(owner_id);
create index listing_geom_gist_idx on listing using gist(geom_point);
create index listing_created_idx on listing(created_at desc) where status = 'active';
create index listing_pnu_idx on listing(parcel_pnu);
```

#### `listing_photo`

```sql
create table listing_photo (
    id char(30) primary key,                            -- lph_... (3-char prefix invariant; was `ph_` in earlier drafts)
    listing_id char(30) not null references listing(id) on delete cascade,
    r2_key text not null,                               -- 'listings/lst_01HXY/photos/p1.jpg'
    thumbnail_r2_key text,
    caption varchar(200),
    display_order int not null default 0,
    width_px int,
    height_px int,
    file_size_bytes bigint,
    content_type varchar(50) not null check (content_type in
        ('image/jpeg', 'image/png', 'image/webp')),
    uploaded_at timestamptz not null default now(),
    deleted_at timestamptz
);

create index listing_photo_listing_order_idx on listing_photo(listing_id, display_order)
    where deleted_at is null;
```

### 5.2 사용자 활동 (Insights BC)

#### `bookmark_listing`

```sql
create table bookmark_listing (
    user_id char(30) not null references "user"(id) on delete cascade,
    listing_id char(30) not null references listing(id) on delete cascade,
    note text,
    created_at timestamptz not null default now(),
    primary key (user_id, listing_id)
);

create index bookmark_listing_user_idx on bookmark_listing(user_id, created_at desc);
```

#### `bookmark_external`

```sql
create table bookmark_external (
    id char(30) primary key,                            -- bme_...
    user_id char(30) not null references "user"(id) on delete cascade,
    target_kind varchar(30) not null check (target_kind in
        ('parcel', 'court_auction', 'manufacturer', 'industrial_complex')),
    target_id varchar(50) not null,                     -- PNU 19자리 또는 다른 식별자
    note text,
    created_at timestamptz not null default now(),
    unique(user_id, target_kind, target_id)
);

create index bookmark_external_user_idx on bookmark_external(user_id, created_at desc);
```

#### `search_history`

```sql
create table search_history (
    id char(30) primary key,                            -- srh_... (3-char prefix per shared-kernel Id<P>)
    user_id char(30) references "user"(id),             -- nullable (비로그인)
    query text not null,
    filters jsonb not null default '{}',
    result_count int not null,
    correlation_id varchar(30) not null,
    created_at timestamptz not null default now()
);

create index search_history_user_time_brin_idx on search_history using brin(created_at);
-- retention: 90일 후 user_id 가명화, 1년 후 삭제 (PIPA)
```

#### `analysis_report`

```sql
create table analysis_report (
    id char(30) primary key,                            -- rpt_...
    user_id char(30) not null references "user"(id) on delete cascade,
    title varchar(200) not null,
    target_pnus char(19)[] not null,                    -- 분석 대상 필지들
    snapshot jsonb not null,                            -- 시점 고정 분석 결과 (R2 데이터 캐시)
    created_at timestamptz not null default now(),
    version bigint not null default 1
);

create index analysis_report_user_idx on analysis_report(user_id, created_at desc);
```

#### `notification`

```sql
create table notification (
    id char(30) primary key,                            -- ntf_...
    user_id char(30) not null references "user"(id) on delete cascade,
    kind varchar(50) not null,                          -- 'bookmark_listing_changed', 'auction_deadline_approaching'
    payload jsonb not null,
    read_at timestamptz,
    created_at timestamptz not null default now()
);

create index notification_user_unread_idx on notification(user_id, created_at desc)
    where read_at is null;
-- retention: 365일 후 자동 삭제
```
