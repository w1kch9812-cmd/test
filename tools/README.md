# tools/

빌드 스크립트, 코드 생성기, 일회성 작업.

## 향후 추가 (sub-project별 단계적)
- `codegen-ts/` — Rust OpenAPI → TS SDK 자동화 스크립트
- `codegen-domain/` — DDD Aggregate 모듈 생성기 (plop 또는 Rust)
- `seed/` — 마스터 데이터 시드 (행정구역, 도로명주소)
- `migrate-helper/` — 무중단 마이그레이션 보조
- `quota-monitor/` — V-World 쿼터 모니터링 CLI
- `data-quality/` — Soda 또는 Great Expectations 룰 (Phase 3+)
- `audit-trail/` — 감사 로그 조회 CLI (Phase 3+)

## 정책
- 각 도구는 별도 워크스페이스 멤버 (`tools/<name>` 폴더)
- 일회성 마이그레이션 스크립트는 실행 후 `_archive/`로
- 운영 도구는 audit log + 권한 검증 필수
