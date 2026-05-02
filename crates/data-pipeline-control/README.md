# data-pipeline-control

데이터 파이프라인 제어 BC (어드민 관리) crate에요.

## 책임

- spec § 5.4 `pipeline_schedule` / `pipeline_run` 테이블 매핑.
- `PipelineSchedule` Aggregate — cron 스케줄 + 워커 실행 락 + optimistic locking.
- `PipelineRun` Aggregate — 실행 1건 + 상태 머신 + `steps` JSONB (단계별 진행 UI 시각화용).
- `RunStatus` enum (5값: running / success / failed / `skipped_unchanged` / aborted).
- `TriggerKind` enum (3값: schedule / manual / event).
- `PipelineRepository` trait — 두 Aggregate 합친 1 trait. 구현체는 sub-project 5 에서.

## 상태 머신

`Running` → 터미널 4 (`Success`, `SkippedUnchanged`, `Failed`, `Aborted`).
터미널 상태 도달 후 `complete_run` / `fail_run` / `abort_run` 등 호출하면
`PipelineError::AlreadyTerminal` 반환해요.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `PipelineScheduleMarker`, `PipelineRunMarker`).
