-- V001_01: Core BC RDS 동적 — user, listing, listing_photo (spec § 5.1)
-- Parcel/Building/IndustrialComplex/Manufacturer는 R2 정적 — 본 파일 범위 밖 (spec § 4)

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
    version bigint not null default 1
);

create index listing_status_idx on listing(status);
create index listing_listing_type_idx on listing(listing_type);
create index listing_owner_idx on listing(owner_id);
create index listing_geom_gist_idx on listing using gist(geom_point);
create index listing_created_idx on listing(created_at desc) where status = 'active';
create index listing_pnu_idx on listing(parcel_pnu);

create table listing_photo (
    id char(30) primary key,                            -- ph_...
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
