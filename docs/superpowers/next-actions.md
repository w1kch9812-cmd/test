# 다음 작업 (Next Actions)

> **갱신일**: 2026-05-04 (SP4-iii-a 종료 직후, commit `2aaf7d9`)
> **목적**: 다음 세션이 컨텍스트 없이도 즉시 시작 가능하도록 우선순위 + 진입점 명시.
> **SSOT**: 본 문서 = 단기 작업큐. 장기 = [`roadmap.md`](./roadmap.md). 진행 현황 = [`memory/project_progress.md`](../../memory/project_progress.md).

---

## 1순위 — SP4-iii-b: data.go.kr 실거래가 + RealTransactionReader (1-2일)

**왜 1순위**: SP4-iii-a 가 만든 `DataGoKrClient` + `Policy::data_go_kr_default` + `pnu_split` + `PgRawCapture` 인프라를 *재사용*. 같은 패턴 답습이라 빠름. `RealTransaction` Aggregate 는 SP2c 에서 이미 구현 (`crates/domain/market/real-transaction`).

**진입점**:

- 도메인: [`crates/domain/market/real-transaction/src/`](../../crates/domain/market/real-transaction/src/) — Aggregate + Reader trait 이미 존재
- 신규 파일: `crates/data-clients/data-go-kr/src/real_transaction/{client.rs,parser.rs,reader.rs}` — `building_register/` 와 같은 모듈 구조
- API endpoint (참고): `data.go.kr` 부동산 실거래가 API
  - 아파트: `getRTMSDataSvcAptTrade`
  - 오피스텔: `getRTMSDataSvcOffiTrade`
  - 단독/다가구: `getRTMSDataSvcSHTrade`
  - 비주거 (산업용): `getRTMSDataSvcNrgTrade` ← **1차 타겟**

**작업 골격**:

1. spec + plan (`docs/superpowers/specs/2026-05-04-sub-project-4-iii-b-real-transaction-design.md` + plan)
2. `RealTransactionRegisterClient::fetch_by_jibun_period(parts, year_month)` — 5분해 파라미터 + `LAWD_CD` (PNU[0..5]) + `DEAL_YMD` (YYYYMM)
3. `parser::parse_real_transactions` — 응답 → `Vec<RealTransaction>` ACL
4. `DataGoKrRealTransactionReader` impl `RealTransactionReader::find_by_pnu_period` (또는 trait 이 정의한 메서드 — 코드 확인 필수)
5. `raw_capture(source = "data_go_kr_tx")` — `parcel_external_data.source` CHECK 에 이미 포함
6. wiremock 6 시나리오 (happy / multi-month / empty / 5xx / malformed / circuit)

**미리 알아둘 것 (SP4-iii-a 발견 lessons 적용)**:

- 한글 라벨 → enum 매핑은 `Other` fallback (외부 스키마 확장에 견고)
- `items.item` 단일/배열/빈 문자열 다형 처리 (`serde_json::Value` match)
- 빈 응답 분기는 V-World 등 secondary fetch 회피로 비용 절감
- `PolygonSrid` required 필드가 도메인에 있으면 V-World 합성 (FU 40 까지)
- `clippy::needless_pass_by_value` 가 헬퍼 fn 의 `Value` 인자에 자주 발동 → `&Value` 받기

---

## 2순위 — SP4-iii-c: 법제처 도시계획 텍스트 (1-2일)

**왜 다음**: `Parcel.zoning` 이 V-World 의 한글 분류만 사용 — 법제처 실제 조례/시행령 텍스트가 정확. ZoningReader port 신규.

**진입점**:

- API: 법제처 Open API (`open.law.go.kr`)
- 신규 crate: `crates/data-clients/korean-law/`
- 도메인: 신규 `ZoningRegulationReader` port — 또는 `Parcel` 의 추가 필드. ADR 필요할 수 있음 (zoning 텍스트가 Aggregate 인지 ValueObject 인지)
- raw_capture source: `"lawmaking"` (이미 CHECK 포함)

**리스크**: 법제처 응답이 HTML/XML 다중 — JSON 파서가 안 듬. 별도 파서 패턴 필요.

---

## 3순위 — SP4-iii-e: R2 PMTiles Reader 6 + FU 40 (2-3일)

**왜 마지막**: 가장 무거움. PMTiles 정적 파일을 S3-호환 R2 버킷에서 fetch + spatial query. `Building.geom` 정확한 footprint 도 여기서 (FU 40).

**진입점**:

- 신규 crate: `crates/data-clients/r2-public-data/`
- ETL 파이프라인 분리: `services/etl-pmtiles-builder` 가 V-World/data.go.kr → PMTiles 빌드 후 R2 upload (별도 서비스)
- 6 Reader: `Parcel::fetch_markers_in_bbox` (현재 honest failure), `Building::fetch_by_id` (FU 42 도 같이), `IndustrialComplex`, `Manufacturer`, `RealTransaction::fetch_markers_in_bbox`, `CourtAuction::fetch_markers_in_bbox`
- FU 40: `Building.geom` 을 V-World `AL_D194_*` (건물 footprint) 또는 PMTiles 에서 가져옴. SP4-iii-a 의 합성 코드 (`reader.rs::fetch_polygon`) 가 polymorphic 으로 분기하도록 변경

**리스크**: PMTiles 파서 (`pmtiles-rs` crate) 가 alpha. 검증 필요. 정적 빌드 경로 결정 필요.

---

## 4순위 — Production 잔여 부채 일괄 정리 (FU 미해소 9건+)

[`roadmap.md` § Spec FU 누적](./roadmap.md) 참조.

특히 production 직전 필수:

- **FU 4 / 6**: BusinessNumber NTS 체크섬 외부 검증 + 사업자유형 코드
- **FU 8**: KsicCode 대분류 letter 강제
- **FU 13**: AuditLog spec § 4.3 ↔ 실제 schema 정렬
- **FU 14**: BVQ/LRQ entity `updated_at` ↔ DB 컬럼 미존재 정합
- **FU 18**: AuthCrate clippy 빚 — `crates/auth/src/verifier.rs` panic + manual_let_else (SP3 잔재)
- **FU 26**: `clippy::disallowed_types` 로 reqwest::Client 직접 호출 차단

---

## 그 다음 단계 (SP4-iii 완전 종료 후)

| SP | 영역 | 추정 |
|---|---|---|
| **SP6** | Frontend (Next.js + React 19, 4-7일) — SP6-i 인증 / SP6-ii 매물 검색 / SP6-iii 북마크 / SP6-iv 알림 | 분해 필요 |
| **SP7** | 관측성 (Grafana + Prometheus + Loki + Tempo + Sentry) — Outbox publisher metrics + Breaker open alert | 2-3일 |
| **SP8** | IaC (Pulumi RDS / R2 / ECS / ALB) | 3-4일 |
| **SP9-12** | 데이터 파이프라인 / AI 어시스턴트 / 검색 / 결제 | TBD |

---

## 환경 체크 (다음 세션 시작 전)

- `cargo --version` → 1.88.0 가 path 에 있는지 (`$env:USERPROFILE\.cargo\bin`)
- `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린 (SP4-iii-a 종료 시점 검증됨)
- `git log --oneline -5` 마지막 commit `2aaf7d9` 확인
- push 권한: `git push origin main` 이 sandbox policy 로 차단될 수 있음 — 사용자 승인 필요
- markdownlint pre-commit hook 활성 — `+`/`*` 로 시작하는 indented 텍스트 금지 (MD004)

---

## SP4-iii-a 가 발견한 명시적 follow-up

| FU | 내용 | 우선순위 |
|---|---|---|
| 40 | `Building.geom` 정확한 footprint (V-World AL_D194 또는 R2 PMTiles) | SP4-iii-e 와 묶음 |
| 41 | `mainPurpsCdNm` / `strctCdNm` 한글 매핑표 28+ 케이스 확장 | low (Other fallback 작동 중) |
| 42 | `BuildingReader::fetch_by_id` (mgmBldrgstPk endpoint) | medium |
| 43 | 캐시 정책 (`expires_at = fetched_at + 30 days`) | medium (SP4-iii 종료 후) |
| 44 | 토지대장 endpoint | SP4-iii-b 와 묶음 검토 |
