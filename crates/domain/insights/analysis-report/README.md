# analysis-report-domain

`AnalysisReport` 도메인 (Insights BC, RDS 동적) crate에요.

## 책임

- spec § 5.2 `analysis_report` 테이블 매핑하는 Aggregate 정의해요.
- 사용자가 다수 필지를 묶어 저장한 분석 리포트.
  - `target_pnus`: ≥1, ≤50 (응답 크기 제한).
  - `snapshot`: `R2` 데이터 시점 캐시 (`JSONB`).
- Optimistic locking (`version: i64`) — `rename` / `update_snapshot` 시 bump.
- `AnalysisReportRepository` trait — 구현체는 sub-project 5에서 추가.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `AnalysisReportMarker`, `Pnu`).

## Spec 후속 (DB 마이그레이션)

Spec § 5.2 `analysis_report` 테이블은 `created_at`만 갖고 있지만, 도메인
Aggregate는 optimistic locking 추적을 위해 `updated_at`를 함께 사용해요.
DB 마이그레이션에 `updated_at timestamptz not null default now()` 컬럼 추가가
필요해요 (`V003_04__analysis_report_updated_at.sql` 등). 본 crate 스코프는
도메인이라 DB 변경은 별도 task로 관리해요.
