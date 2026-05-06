# ADR-0014: 지도 base layer (전국 필지/건물 polygon) — 보류

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Accepted (defer) |
| 결정자 | 사용자 |
| 컨텍스트 | SP4-iii-e 가 제안한 R2 PMTiles base layer 의 SSS 적합성 검토 |

## 컨텍스트

SP4-iii-e 1차 (commit `9d8a513`) 는 Cloudflare R2 위 PMTiles 정적 파일을
reader 가 fetch 해 *전국 필지 + 건물 footprint* 를 base layer 로 표시하는
방향으로 설계됐습니다. 그러나 SP6-iv 종료 직후 근본 SSS 검토에서 다음 결함이
드러났습니다:

1. **SSOT 위반 (§ 6)** — V-World/data.go.kr → ETL 빌더 → PMTiles → R2 = 사본 3 개.
   PostGIS 도 별도. 진실의 위치가 모호함.
2. **freshness lag** — batch 빌드 (시간/일 단위). 변경된 필지가 즉시 반영 안 됨.
3. **자동강제 부재 (§ 2)** — ETL cron 깨지면 stale data 침묵. 시스템이 차단 안 함.
4. **scope creep 의심** — 1.4억 필지 polygon 전국 표시는 B2B 산업용 부동산 사용자
   needs 검증 미완. 실제 흐름 = 특정 산단/지역 한정 검색.

현재 작동 중인 마커 흐름 (SP6-ii):

```text
listings 테이블 (PostGIS GIST) → ListingRepository.find_card_summaries_in_bbox
  → /listings?bounds=... → Naver Maps gl 핀
```

이는 *우리 매물* 만 표시. 전국 base layer 는 별도 문제.

## 결정

**SP4-iii-e R2 PMTiles 방향 보류 (defer)**. 지도 base layer 를 *지금 추가하지
않음*. Naver Maps 가 제공하는 default base layer (도로/지명) 만 사용.

폐기 안 한 자산:

- `crates/data-clients/r2-public-data` crate (R2Client + PMTiles header parser)
  는 *유지* — 향후 R2 가 cache layer 로 도입될 때 (B2C 트래픽 폭증 시) 재활용.
- `Policy::r2_default` 는 유지.
- 기존 `R2ParcelReader` 의 honest failure stub 도 유지.

## 대안

| 안 | 평가 |
|---|---|
| **A. R2 PMTiles + ETL 빌더 (FU 60)** | ❌ SSOT 위반 / batch lag / build cost / 사용자 needs 미검증 |
| **B. martin (Rust MVT tile server) + PostGIS** | 🟡 SSOT 유지 + realtime + Cloudflare 캐시 — 단 PostGIS 에 1.4억 필지 적재 필요 (디스크 100GB+, V-World 데이터 자체 보유 책임). 트래픽 증가 시 재고 |
| **C. defer (본 결정)** | ✅ 현재 사용자 needs 충족. base layer 가 진짜 필요한지 데이터로 결정 (사용자 행동 추적) |
| **D. V-World WMS realtime overlay** | 🟡 매물 hover 시 1 폴리곤만 V-World API 로 fetch (이미 가능). bbox panning 마다 호출은 quota 초과 — 부분적 가능 |

## 결과

**즉시**:

- SP4-iii-e 1차 commit (`9d8a513`) 는 *foundation only* 로 동결.
- R2BuildingReader / BuildingFootprintSource / ICReader 미구현 — FU 60 placeholder
  유지하되 *현재 우선순위 아님* 으로 표시.
- FU 40 (`Building.geom` 정확한 footprint) 는 V-World `AL_D194_*` 레이어 단건
  호출 + 캐시 (Redis FU 28) 로 별도 실현 가능 — PMTiles 미의존.

**조건부 활성화 (재검토 트리거)**:

1. 사용자 행동 추적: 지도 panning 동작이 검색 흐름의 30% 이상 차지 → base layer 필요성 검증
2. 트래픽 1만 DAU 돌파 → Cloudflare 캐시 layer 정당화
3. 산단/시군구 단위 *경계 polygon* 표시 needs → R2 IndustrialComplex Reader 만 부분 구현

**관련 폐기**:

- spec `2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md` 의 § 1-7 = *foundation
  only* 이며 § 4.4 (R2BuildingReader) / § 4.5 (R2IndustrialComplexReader) 는
  본 ADR 의 결정으로 보류.
- FU 40 / 60 / 61 / 67 = 본 ADR 의 *재검토 트리거* 충족 시 재활성화.

## SSS 7기둥

| 기둥 | 적용 |
|---|---|
| 6 SSOT | 사본 0 — V-World/data.go.kr 가 그대로 진실. 사본은 만들 때만 만든다 |
| 4 안전성 | 미실현 코드 0 (R2 readers 가 honest failure 명시) — silent stale 위험 차단 |
| 7 명확성 | 본 ADR 가 *왜 안 만드는지* 명시 — 다음 세션 / 새 멤버가 의문 가질 때 답이 git 에 있음 |

## 참고

- spec [`2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md`](../superpowers/specs/2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md)
- ADR [`0013-listing-search-naver-maps.md`](./0013-listing-search-naver-maps.md)
- AGENTS.md § 8 SSOT
