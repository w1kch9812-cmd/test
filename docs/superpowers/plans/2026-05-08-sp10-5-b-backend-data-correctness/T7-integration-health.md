# T7: AppState/lib.rs Split + ReadinessResponse + Integration Test Rewrite

**Goal:** `services/api` 를 router/state factory 로 분리 (lib.rs + state.rs) → 통합 테스트 가 실 router 호출 가능. `/healthz/ready` 의 `HealthResponse` 단순 구조를 `ReadinessResponse { status, checks }` nested 로 확장. `services/api/tests/sp10_panel_endpoints.rs` 의 핸들러 재구현 path 를 *실 router* 호출로 재작성.

**Spec SSOT:** §10.5 (Health Shape), §11 통합 변경 표 (services/api split, sp10_panel_endpoints.rs 재작성), §13 T7 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**Files:**

- Create: `services/api/src/state.rs`
- Create: `services/api/src/lib.rs`
- Create: `services/api/tests/sp10_backend_data_correctness.rs`
- Modify: `services/api/src/main.rs` (lib.rs 의 `app_router` 호출하는 thin entry point)
- Modify: `services/api/src/routes/health.rs` (lines 45-125 ReadinessResponse 추가)
- Modify: `services/api/tests/sp10_panel_endpoints.rs` (lines 29-36, 148-250 — spawn_test_app → app_router)
- Modify: `services/api/Cargo.toml` ([lib] + [[bin]] sections)

---

## Plan Parts

Detailed step bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - AppState And Readiness Response](./T7-integration-health.part-01-state-readiness.md)
- [Part 02 - Router Factory And Panel Endpoint Tests](./T7-integration-health.part-02-router-panel-tests.md)
- [Part 03 - Backend Correctness Integration And Acceptance](./T7-integration-health.part-03-integration-acceptance.md)
