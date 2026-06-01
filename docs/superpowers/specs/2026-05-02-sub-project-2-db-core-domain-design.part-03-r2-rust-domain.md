# Sub-project 2 DB Core Domain Design - Part 03: R2 Static Data And Rust Domain Code

Parent index: [Sub-project 2 DB Core Domain Design](./2026-05-02-sub-project-2-db-core-domain-design.md).

## 7. R2 정적 데이터 구조

### 7.1 두 버킷

```
gongzzang-public-data/      공개 정적 데이터 (CDN 배포, 사용자 다운로드)
gongzzang-raw-archive/      외부 API raw 응답 (감사용, 비공개)
```

### 7.2 `gongzzang-public-data` 구조

```
parcels/
  {sido_code}/
    parcels.pmtiles             시도별 PMTiles (서울 ~2GB)
    last_sync.json              마지막 sync 메타 (timestamp, hash, source)

buildings/
  {sido_code}/
    buildings.pmtiles
    last_sync.json

industrial-complexes/
  index.geojson                 전국 1천 개 (작음, 한 파일)
  details/
    {complex_id}.json

real-transactions/
  by-month/
    {year}-{month}.json         예: 2026-05.json (전국 신규)
  by-region/
    {sido_code}/
      {sigungu_code}/
        {year}.json             분석용

court-auctions/
  active.json                   진행 중 경매 (매일 갱신)
  history/
    {year}/{month}.json         종료 경매 (월별 archive)

manufacturers/
  by-industry/
    {ksic_code}.json
  by-region/
    {sido_code}.json

laws/
  index.json                    법령 목록
  texts/
    {law_id}.json               법령 본문 (조항별)
  embeddings/                   (Phase 3+)
    {law_id}.bin

masters/
  administrative-divisions.json
  road-addresses.json
  ksic-codes.json
  zoning-codes.json
  land-use-types.json

listings/
  {listing_id}/
    photos/
      p1.jpg, p1_thumb.jpg, p2.jpg, ...   매물 사진 (presigned URL 업로드)
```

### 7.3 `gongzzang-raw-archive` 구조

```
vworld/
  {date}/                                          예: 2026-05-02/
    {request_id}.json.gz                            V-World 응답 raw
data-go-kr/
  {date}/
    {request_id}.json.gz
korean-law/
  {date}/
    {request_id}.json.gz
nice-identity/                                     (Phase 3+)
  {date}/
    {request_id}.json.gz                            인증 요청 raw (PII 마스킹 후)
```

retention: 7년 (PIPA + ISMS-P + 분쟁 시 증빙). R2 Object Lock immutable.

### 7.4 갱신 전략

| 데이터 | 갱신 주기 | 변경 감지 | shard 단위 |
|--------|---------|----------|---------|
| 필지 PMTiles | 분기 (어드민 조정 가능) | V-World 응답 해시 vs R2 hash | 시도 17개 |
| 건축물 PMTiles | 분기 | data.go.kr `lastUpdtDt` + hash | 시도 17개 |
| 산업단지 | 연 + 이벤트 | 정부 공시 (수동 트리거) | 단일 |
| 실거래 | 일 | API에 `dealYmd` 필터, 신규분 append | 월별 분할 |
| 경매 active | 일 | 사건번호별 갱신일 | 단일 |
| 경매 history | 월 (1일 03:00) | active → history 이전 | 월별 |
| 법령 | 변경 이벤트 | 법제처 webhook 또는 polling | 법령별 |
| 마스터 (행정구역 등) | 분기 | 정부 표준 코드 변경 | 단일 |

### 7.5 Shard 단위 hash 비교

워커 흐름 (시도 17개 예):
```
1. V-World 호출 → 전국 필지 응답
2. 시도별로 분할 → 17 그룹
3. 각 그룹 PMTiles 생성 → 17 hash 계산
4. R2의 sido_11/last_sync.json hash와 비교
   - 같음 → 그 시도 PMTiles 업로드 skip
   - 다름 → 업로드 + last_sync.json 갱신 + Cloudflare CDN purge
5. pipeline_run.output_hashes에 17 hash 모두 기록
```

→ 4천만 필지 중 *서울만 변경*이면 *서울 PMTiles만* 재업로드.

### 7.6 멱등성 (Postgres advisory lock)

```sql
-- 워커 시작 시
select pg_try_advisory_lock(hashtext('pipeline:parcel_sync'));
-- 1 = lock 획득 → pipeline_run INSERT (status='running')
-- 0 = 다른 워커 실행 중 → skip + log
```

`pipeline_schedule.running_lock_acquired_at` + `running_worker_id`로 모니터링 (어드민 UI에서 stuck 워커 감지 + 강제 해제).

---

## 8. Rust 도메인 코드 구조

### 8.1 워크스페이스

```
crates/
├── domain/
│   ├── core/
│   │   ├── user/                    RDS 동적
│   │   ├── listing/                 RDS 동적
│   │   ├── parcel/                  R2 정적 (Reader trait)
│   │   ├── building/                R2 정적
│   │   ├── industrial-complex/      R2 정적
│   │   ├── manufacturer/            R2 정적
│   │   └── shared-kernel/           Pnu, Money, Area, Geometry, AdminDivision 등
│   │
│   ├── market/
│   │   ├── real-transaction/        R2 정적 (read-only)
│   │   ├── court-auction/           R2 정적
│   │   ├── inquiry/                 RDS 동적 (Phase 2+ 자리)
│   │   └── subscription/            RDS 동적 (Phase 2+ 자리)
│   │
│   ├── regulation/
│   │   ├── law/                     R2 정적
│   │   └── regulation/              R2 정적 (자리)
│   │
│   ├── insights/
│   │   ├── bookmark/                RDS 동적 (Listing FK + External polymorphic)
│   │   ├── search-history/          RDS 동적
│   │   ├── analysis-report/         RDS 동적
│   │   └── notification/            RDS 동적
│   │
│   └── audit/
│       └── audit-log/               RDS 동적 (immutable)
│
├── operations/                      신규 — 어드민 운영 도메인
│   ├── admin-action/
│   ├── business-verification/
│   ├── listing-review/
│   ├── listing-report/
│   ├── featured-content/
│   └── system-alert/
│
├── data-pipeline-control/           신규 — 파이프라인 schedule + run
│   ├── schedule/
│   ├── run/
│   └── steps/
│
├── db/                              SQLx + PostGIS Repository (sub-project 5에서 본격 구현)
├── data-clients/                    R2 reader + 외부 API HTTP (sub-project 4에서 본격)
├── geo/, auth/, cache/, observability/, circuit-breaker/, api-types/, embedding/
```

### 8.2 값 객체 (shared-kernel)

```rust
// crates/domain/core/shared-kernel/src/pnu.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Pnu(String);

impl Pnu {
    pub fn try_new(s: &str) -> Result<Self, PnuError> {
        if s.len() != 19 || !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(PnuError::InvalidFormat);
        }
        // 추가: 시도/시군구 코드 검증
        Ok(Self(s.to_owned()))
    }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn sido_code(&self) -> &str { &self.0[0..2] }
    pub fn sigungu_code(&self) -> &str { &self.0[0..5] }
}

// crates/domain/core/shared-kernel/src/money.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub struct Money(i64); // KRW 단위, 음수 불가능

impl Money {
    pub fn try_new(krw: i64) -> Result<Self, MoneyError> {
        if krw < 0 { return Err(MoneyError::Negative); }
        Ok(Self(krw))
    }
    pub fn krw(&self) -> i64 { self.0 }
}

// 다른 값 객체:
// Area (㎡), BusinessNumber (10자리), BrokerLicense, RoadAddress, JibunAddress,
// Email, PhoneKr, AdminDivision, ListingTitle, Description, ULID 헬퍼 등
```

### 8.3 Aggregate 예시 (Listing)

```rust
// crates/domain/core/listing/src/entity.rs
pub struct Listing {
    pub id: ListingId,
    pub owner_id: UserId,
    pub parcel_pnu: Pnu,
    pub listing_type: ListingType,
    pub transaction_type: TransactionType,
    pub price: Money,
    pub deposit: Option<Money>,
    pub monthly_rent: Option<Money>,
    pub area: Area,
    pub title: ListingTitle,
    pub description: Description,
    pub status: ListingStatus,
    pub contact_visibility: ContactVisibility,
    pub view_count: u64,
    pub bookmark_count: u64,
    pub geom_point: Option<Point>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub version: i64,
}

// 상태 머신
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingStatus {
    Draft,
    PendingReview,
    Active,
    Sold,
    Expired,
    Rejected,
}

impl Listing {
    pub fn submit_for_review(&mut self) -> Result<(), ListingError> {
        match self.status {
            ListingStatus::Draft => {
                self.status = ListingStatus::PendingReview;
                self.version += 1;
                Ok(())
            }
            _ => Err(ListingError::InvalidTransition {
                from: self.status, to: ListingStatus::PendingReview
            })
        }
    }
    // approve, reject, mark_sold, expire 등
}

// crates/domain/core/listing/src/repository.rs
#[async_trait::async_trait]
pub trait ListingRepository: Send + Sync {
    async fn find(&self, id: &ListingId) -> Result<Option<Listing>, RepoError>;
    async fn find_markers_in_bbox(&self, bbox: &BoundingBox) -> Result<Vec<ListingMarker>, RepoError>;
    async fn save(&self, listing: &Listing) -> Result<(), RepoError>;
}
```

### 8.4 R2 Reader (Parcel)

```rust
// crates/domain/core/parcel/src/entity.rs
pub struct Parcel {
    pub pnu: Pnu,
    pub admin: AdminDivision,
    pub road_address: Option<RoadAddress>,
    pub jibun_address: JibunAddress,
    pub land_use_type: LandUseType,
    pub area: Area,
    pub official_land_price_per_m2: Option<Money>,
    pub zoning: Zoning,
    pub geom: Polygon,
    pub fetched_at: DateTime<Utc>,
}

// Reader trait (Repository와 다름 — read-only)
#[async_trait::async_trait]
pub trait ParcelReader: Send + Sync {
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError>;
    async fn fetch_markers_in_bbox(&self, bbox: &BoundingBox) -> Result<Vec<ParcelMarker>, ReaderError>;
}

// 구현체 (sub-project 4에서):
// crates/data-clients/r2-public-data/src/parcel_reader.rs
// - PMTiles에서 spatial query
// - 시도 코드 추출 → 해당 시도 PMTiles만 fetch
// - moka L1 + Valkey L2 캐시
```

---
