-- V003_04: analysis_report.updated_at 컬럼 추가
-- spec § 5.2 누락 컬럼 보강 (Sub-project 2c FU 9)
--
-- AnalysisReport Aggregate (crates/domain/insights/analysis-report) 는
-- optimistic locking 버전 bump 시 `updated_at` 도 함께 갱신해요. 그러나
-- spec § 5.2 의 analysis_report 테이블 정의에는 `created_at` 만 있어
-- DB-도메인 스키마가 어긋났어요. 본 마이그레이션이 그 격차를 해소해요.
--
-- 기존 row 는 `created_at` 값으로 백필 (생성 직후 = 갱신된 적 없음).

alter table analysis_report
    add column updated_at timestamptz not null default now();

update analysis_report set updated_at = created_at where updated_at >= created_at;
