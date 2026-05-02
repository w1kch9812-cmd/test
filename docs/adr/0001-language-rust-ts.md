# ADR-0001: 언어 — Rust + TypeScript

| | |
|---|---|
| 작성일 | 2026-05-01 |
| 상태 | Accepted |
| 결정자 | 운영자 |

## 컨텍스트

산업용 부동산 정보 플랫폼 (옵션 A 데이터 플랫폼) 백엔드와 프론트엔드 언어 선택.
SSS 엔터프라이즈 + 메모리 안전 + 성능 + 동시성 + 한국 시장 대상 데이터 분석 부하 (한국 전체 필지 4,000만, 건축물 700만, 제조업체 30만).

## 결정

- **백엔드**: Rust 1.88+ (2026-05-02 amendment 2 — Walking Skeleton T2에서 sqlx 0.8 + rustls transitive deps (home 0.5.12, icu_* 2.2.0)로 1.85 → 1.88. 원안 1.83+, amendment 1: edition2024로 1.85)
- **프론트엔드**: TypeScript 5.7 + Next.js 16 + React 19

백엔드는 모든 도메인 로직, 외부 API 통합, 데이터 처리 담당.
Next.js는 UI 렌더링 + 얇은 BFF 프록시 (인증 검사 + Rust API 호출). 비즈니스 로직 0줄.

## 대안

- **Kotlin / Spring Boot 3**: 한국 엔터프라이즈 검증, ISMS-P 사례 풍부. 단점: JVM 메모리 무거움(1GB+), GC 일시정지, 대량 공간 데이터 처리 비효율
- **Go + Gin/Echo**: Cloud Native 친화, 빠른 컴파일. 단점: data race 가능, 메모리 안전 일부만, 제너릭 제한
- **Node.js + NestJS 풀스택**: 단일 언어, 빠른 개발, 풀 타입 공유. 단점: CPU 집약 작업 약함, 동시성 한계, 대량 데이터 분석 부하 약함

## 결과

- 긍정: 메모리 안전(컴파일 보장) + 성능(C++ 수준) + 동시성(Tokio + ownership으로 data race 차단) + 공급망 보안(cargo-audit) + PostGIS 친화(postgis crate, geo-types) + WASM 미래성
- 부정: 학습 곡선 가파름(borrow checker, lifetime), 빌드 시간 (첫 빌드 5-10분), 한국 인력 풀 작음, 일부 한국 SDK(NICE 본인인증) 직접 구현 필요
- 영향 영역: 모든 `services/`, `crates/` (백엔드), `apps/`, `packages/` (프론트)

## 재검토 트리거

- Rust 채용 6개월 이상 실패 시
- 백엔드 빌드 시간이 CI에서 30분 초과 시
- 도메인 SDK 직접 구현이 전체 작업의 30% 초과 시

## 참조

- → @docs/conventions/rust.md
- → @docs/conventions/typescript.md
- → @docs/sss-charter.md (안전성·성능 기둥)
