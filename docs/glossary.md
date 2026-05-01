# 도메인 용어 사전 (Glossary)

> *Ubiquitous Language* SSOT. 한국어(사용자 노출) ↔ 영어(코드) 1:1 매핑.
> 코드/문서/UI 모두 이 매핑을 따름. 위반 시 CI lint 차단 (sub-project 5+).

---

## 1. 부동산 / 토지

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| 필지 | `Parcel` | 토지의 등록 단위 (PNU 19자리로 식별) | NOT `Land` (혼동), NOT `Lot` |
| PNU | `Pnu` (값 객체) | 19자리 필지고유번호 (시도2+시군구3+법정동5+지번4+지목2+산림1+예비2) | Newtype |
| 매물 | `Listing` | 거래 대상 부동산 (공장/창고/사옥 등) | NOT `Property` (일반 단어, 헷갈림) |
| 지목 | `LandUseType` | 토지 분류 (대/공장용지/창고용지 등 28종) | data.go.kr 코드표 사용 |
| 용도지역 | `Zoning` | 도시계획법상 토지 용도 분류 | NOT `Zone` (애매), V-World LT_C_UQ111-114 |
| 지구단위계획 | `DistrictPlan` | 도시계획법상 세부 계획 | V-World UPISUQ161 |
| 도시계획시설 | `UrbanFacility` | 9종 도시계획시설 | V-World UPISUQ151-159 |
| 개발제한구역 | `DevelopmentRestrictionZone` | 그린벨트 | V-World UPISUQ171 |
| 건폐율 | `BuildingCoverageRatio` (BCR) | 대지 면적 대비 건축 면적 비율 | % 단위 |
| 용적률 | `FloorAreaRatio` (FAR) | 대지 면적 대비 연면적 비율 | % 단위 |

## 2. 건축물 / 시설

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| 건축물대장 | `BuildingRegister` | 국토부 건축물 공식 등록부 | 4종: 표제부/총괄/전유부/노출 |
| 표제부 | `BuildingTitleSection` | 건축물대장 기본 정보 | mgmBldrgstPk → string 저장 (BigInt 손실 방지) |
| 총괄표제부 | `BuildingRecapTitle` | 집합건물 통합 정보 | |
| 전유부 | `BuildingExclusiveSection` | 호실별 정보 | |
| 연면적 | `TotalFloorArea` | 건물 총 면적 (㎡) | NOT `BuildingSize` |
| 공장 | `Factory` | 제조업 시설 | |
| 창고 | `Warehouse` | 보관/물류 시설 | |
| 물류센터 | `LogisticsCenter` | 대규모 물류 거점 | |
| 산업단지 | `IndustrialComplex` | 정부 지정 산업 집적 구역 | |
| 지식산업센터 | `KnowledgeIndustryCenter` (KIC) | 도시형 공장 + 사무실 복합 | NOT `IT 빌딩` |
| 사옥 | `OfficeBuilding` | 자가/임차 사무 빌딩 | |

## 3. 거래 / 가격

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| 실거래가 | `RealTransactionPrice` | 국토부 신고 실거래 가격 (만원 단위) | data.go.kr |
| 공시지가 | `OfficialLandPrice` | 표준지/개별 공시지가 (원/㎡) | |
| 공시가격 | `OfficialPrice` | 토지 + 주택 통칭 | |
| 평당가 | `PricePerPyeong` | ㎡당가 환산 (1평 = 3.305785㎡) | UI에서만 표시, DB는 ㎡ |
| 매매 | `Sale` | 소유권 이전 거래 | NOT `Trade` |
| 임대 | `Lease` | 임차 계약 | |
| 경매 | `CourtAuction` | 법원 경매 | NOT `Auction` (개인 경매와 구분) |
| 분양 | `Subscription` (분양 컨텍스트) | 신규 부동산 분양 | 결제의 Subscription과 다름 — 컨텍스트 분리 |

## 4. 사용자 / 권한

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| 매수자 | `Buyer` | 매물 검색·구매 의도 사용자 | |
| 매도자 | `Seller` | 매물 등록 사용자 | 일반인 X (사업자만) |
| 공인중개사 | `Broker` | 자격증 보유 중개인 | NOT `Agent`, NOT `Realtor` |
| 시행사 | `Developer` | 부동산 개발 사업자 | NOT `Builder` |
| 기업 회원 | `EnterpriseMember` | 법인 사용자 (다수 임직원 계정 가능) | |
| 운영자 | `Operator` | 내부 콘텐츠 큐레이터 | RBAC |
| 관리자 | `Admin` | 시스템 관리자 (최상위) | RBAC |
| 사업자등록번호 | `BusinessNumber` (값 객체) | 10자리 사업자 식별 (XXX-XX-XXXXX) | Newtype |
| 공인중개사 자격증 | `BrokerLicense` | 자격증 번호 + 사무소 정보 | |
| 사용자 | `User` | 가입한 모든 계정 | Aggregate Root |

## 5. 인증 / 권한

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| 본인인증 | `IdentityVerification` | NICE 본인확인 | NOT `Auth` (인증 일반과 구분) |
| 권한 | `Permission` | 단일 행위 가능 여부 | |
| 역할 | `Role` | 권한 묶음 | RBAC |
| 세션 | `Session` | 인증된 사용자 컨텍스트 | Redis 저장 |
| 토큰 | `Token` | OIDC JWT (Zitadel 발급) | |

## 6. 공공 데이터 / 외부 API

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| V-World | `VWorld` | 공간정보산업진흥원 공간 데이터 API | |
| 공공데이터포털 | `OpenDataPortal` | data.go.kr 통합 API | NOT `PublicData` (일반 단어) |
| 법제처 | `KoreanLawCenter` | open.law.go.kr 법령 정보 | |
| 토지이음 | `EUM` | 토지·도시계획 정보 (V-World로 대체) | 직접 스크래핑 금지 |

## 7. 분석 / 컨텐츠

| 한국어 | 영문 (코드) | 정의 | 비고 |
|--------|------------|------|------|
| 분석 리포트 | `AnalysisReport` | 사용자 생성/저장 분석 결과 | |
| 즐겨찾기 | `Bookmark` | 매물/회사 저장 | NOT `Favorite` |
| 검색 이력 | `SearchHistory` | 사용자 검색 로그 | PIPA 마스킹 |
| 알림 | `Notification` | 즐겨찾기 변경/시세 변동 알림 | |
| 문의 | `Inquiry` | 매물 → 매도자 연락 | Phase 1: 연락처 노출, Phase 2+: 메시징 |

## 8. 시스템 / 인프라

| 한국어 | 영문 (코드) | 정의 |
|--------|------------|------|
| 감사 로그 | `AuditLog` | 모든 데이터 변경 immutable 기록 |
| 도메인 이벤트 | `DomainEvent` | DDD 비즈니스 이벤트 (Outbox 패턴) |
| 회로 차단기 | `CircuitBreaker` | 외부 API 장애 시 빠른 실패 |
| 멱등성 키 | `IdempotencyKey` | 중복 요청 차단용 헤더 |
| 좌표계 | `SRID` | EPSG 코드 (4326/5179/5186/3857) |
| 마이그레이션 | `Migration` | DB 스키마 변경 (sqlx migrate) |

---

## 사용 규칙

1. **코드** — 영문 (`Listing`, `Pnu`, `Broker`)
2. **API URL** — kebab-case (`/v1/listings`, `/v1/parcels/{pnu}`)
3. **API 응답 필드** — camelCase (`createdAt`, `pricePerSqm`)
4. **DB 컬럼** — snake_case 단수 (`listing_id`, `business_number`)
5. **사용자 노출 한국어** — 위 표의 한국어 그대로 ("매물", "필지")
6. **에러 메시지 (해요체)** — 한국어 단어 우선

## 금지 단어

| ❌ 사용 금지 | ✅ 대신 |
|------------|--------|
| `Property` (코드) | `Listing` |
| `Land` (코드) | `Parcel` |
| `Lot` (코드) | `Parcel` |
| `Realtor`, `Agent` | `Broker` |
| `Builder` | `Developer` |
| `Auction` (모호) | `CourtAuction` |
| `Favorite` | `Bookmark` |
| `BizNo`, `BRN` | `BusinessNumber` |
| "물건" (UI) | "매물" |
| "부동산" (UI에서 매물 의미로) | "매물" |

## 추가 규칙

- 새 도메인 용어 발견 시 *코드 작성 전* 이 문서에 추가
- 영문/한국어 둘 다 *유일*해야 (1:1)
- "확장된 의미" 필요 시 새 영문 단어 (예: `KnowledgeIndustryCenter` ≠ `OfficeBuilding`)
- 위반 자동 검출: CI grep + custom Biome rule (sub-project 5+)
