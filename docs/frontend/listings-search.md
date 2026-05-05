# Listings 검색 화면 — 운영 가이드

> SP6-ii. 디버깅 / 데이터 source / Naver Maps quota / 자주 발생하는 이슈.

## 1. 화면 흐름

```
/login (SP6-i) → /listings (SP6-ii)
                  ↓
                  proxy.ts auth gate (sid → access_token Bearer)
                  ↓
                  GET /api/proxy/listings?bounds=&types=&page=
                  ↓
                  backend GET /listings → ListingRepository::find_card_summaries_in_bbox
                  ↓
                  PostGIS ST_Within(geom_point, ST_MakeEnvelope) + filter + page
```

## 2. 환경 변수

```
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=<NCP Maps Client ID>
```

NCP 가입 후 Maps 등록. Free tier: 10만 호출/월. 초과 시 SP7-i 의 alert.

## 3. 자주 발생하는 이슈

| 증상 | 원인 후보 | 확인 |
|---|---|---|
| 지도 안 뜸 | `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` 미설정 | DevTools Console 의 `Naver Maps SDK failed to load` |
| 매물 0건 | DB 에 매물 없음 / 모든 매물 `status != active` | psql `SELECT count(*) FROM listing WHERE status='active'` |
| 필터 무시됨 | URL query 와 store 동기화 깨짐 | DevTools Network 의 `?types=` 확인 |
| 무한 스크롤 안 됨 | IntersectionObserver sentinel 이 viewport 밖 | DevTools Elements 의 sentinel div 확인 |
| 폰트 fallback (Pretendard 안 보임) | `next/font/local` 의 woff2 file 누락 | `apps/web/public/fonts/` 의 4 file 확인 |

## 4. 데이터 source

- DB `listing` 테이블 (V001) — `status='active'` 만 표시
- 외부 API (V-World / data.go.kr) sync 는 SP6-data-sync 가 책임
- 사진 (`thumbnail_url`) 은 SP6-iii 의 listing-photo 테이블 join 으로 채움

## 5. 주요 컴포넌트

| File | 역할 |
|---|---|
| `app/(authenticated)/listings/page.tsx` | 통합 layout (3-column grid) |
| `components/listings/listing-map.tsx` | Naver Maps + 핀 + mapReady race fix |
| `components/listings/listing-card-list.tsx` | 카드 + 무한 스크롤 + skeleton |
| `lib/listings/use-listings-query.ts` | 단일 useInfiniteQuery hook (캐시 공유) |
| `stores/listings.ts` | Zustand: bounds + filters + selectedListingId |

## 6. CSP 정책 (SP6-ii 추가)

`apps/web/app/api/proxy/proxy.ts` 의 CSP header:

```
script-src 'self' 'nonce-X' 'strict-dynamic' https://oapi.map.naver.com
img-src 'self' data: blob: https://*.map.naver.com https://*.pstatic.net
connect-src 'self' <BACKEND> <ZITADEL> https://*.map.naver.com https://*.naver.com
```

Naver Maps SDK 가 외부 domain 을 호출하므로 strict-dynamic + explicit allowlist 를 동시 적용합니다 (CSP3 + CSP2 호환).

## 7. 디버깅 체크리스트

```bash
# 1. backend 매물 데이터 확인
psql $DATABASE_URL -c "SELECT count(*) FROM listing WHERE status='active' AND geom_point IS NOT NULL"

# 2. backend /listings endpoint 직접 테스트 (인증 토큰 필요)
curl -H "Authorization: Bearer <jwt>" \
  "http://localhost:8080/listings?bounds=33,124,39,132&page=0&size=5" | jq

# 3. frontend dev 시작
docker compose -f infra/zitadel/docker-compose.yml up -d
cargo run -p api  # 별도 터미널
pnpm --filter=@gongzzang/web dev

# 4. 브라우저 → http://localhost:3000/login → admin → /listings
```

## 8. 미래 sub-project 자리

- SP6-iii: 매물 상세 (`/listings/:id`) + 즐겨찾기 toggle + 사진 갤러리
- SP6-iv: 매물 등록 (broker 전용)
- SP6-data-sync: V-World / data.go.kr / 공공 API → DB 자동 sync
- SP6-search-region: 지역 검색 (Naver / 카카오 주소 검색 API 통합)

## 9. Spec / Plan / ADR

- Spec: [docs/superpowers/specs/2026-05-05-sub-project-6-ii-listing-search-design.md](../superpowers/specs/2026-05-05-sub-project-6-ii-listing-search-design.md)
- Plan: [docs/superpowers/plans/2026-05-05-sub-project-6-ii-listing-search.md](../superpowers/plans/2026-05-05-sub-project-6-ii-listing-search.md)
- ADR-0013: [docs/adr/0013-listing-search-naver-maps.md](../adr/0013-listing-search-naver-maps.md)
