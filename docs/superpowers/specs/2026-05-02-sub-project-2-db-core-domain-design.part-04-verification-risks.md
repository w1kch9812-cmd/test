# Sub-project 2 DB Core Domain Design - Part 04: Admin Preview, Verification, Risks, And References

Parent index: [Sub-project 2 DB Core Domain Design](./2026-05-02-sub-project-2-db-core-domain-design.md).

## 9. 어드민 UI 통합 원칙 (sub-project 6 미리보기)

### 9.1 9 화면 구조

```
admin-web/
├── /dashboard               전체 헬스 + 알림 + 비용 위젯 + 큐 사이즈
├── /users                   목록·검증 큐 (탭) → /users/{id} (컨텍스트)
├── /listings                목록·검수·신고 (탭) → /listings/{id} (컨텍스트) + /listings/review-queue
├── /pipelines               목록·진행 → /pipelines/{id} (단계별 진행 시각화)
├── /content                 광고/추천 (Phase 2+)
├── /observability           Grafana embed (서비스 맵 / 트레이스 / 메트릭 / 로그 / 에러 — 탭)
├── /audit                   전역 audit 검색 (대상별 audit는 컨텍스트 페이지에)
├── /costs                   비용 (Phase 3+)
└── /settings                Feature flag, 권한, 시스템 설정
```

### 9.2 공유 위젯 (어디든 embed)

| 위젯 | 데이터 소스 | 용도 |
|------|---------|------|
| `AuditLogWidget` | RDS `audit_log` 필터 | 이 대상의 변경 이력 |
| `MetricsWidget` | Grafana API | 이 대상의 메트릭 |
| `AlertsWidget` | RDS `system_alert` | 이 대상 관련 알림 |
| `RelatedActionsWidget` | RDS query + admin_action insert | 운영 액션 (검증/검수/신고처리) |
| `TraceLinkWidget` | Grafana Tempo URL | 이 대상 관련 트레이스 |

### 9.3 컨텍스트 중심 예시 — `/listings/{id}`

한 화면에:
- 매물 본체 (RDS `listing`)
- 매물 사진 (R2 + presigned URL via `listing_photo`)
- 등록자 (UserCard 위젯)
- 위치 정보 (R2 `parcel/...` reader)
- 신고 (`listing_report` 필터)
- audit log (위젯)
- 메트릭 (위젯)
- 운영 액션 (승인/거부/일시정지/추천)

→ 운영자가 *3초 내 판단 + 처리*. 별도 페이지 왕복 X.

---

## 10. 파이프라인 진행 시각화

### 10.1 두 종류 시각화

| 시각화 | 어디 | 데이터 |
|------|------|------|
| **서비스 맵** (이미지의 Maple 식) | Grafana Tempo embed in `/observability` | OTel 트레이스 자동 |
| **파이프라인 진행** (단계별 카드) | 자체 admin-web UI in `/pipelines/{id}` | RDS `pipeline_run.steps` JSONB |
| **분산 추적** (한 요청 호출 체인) | Grafana Tempo embed | OTel 트레이스 |
| **메트릭·로그·에러** | Grafana embed | Prometheus/Loki/Sentry |

### 10.2 파이프라인 단계 시각화 데이터 흐름

```
Worker 실행 시:
1. pipeline_run INSERT (status='running', steps=[])
2. 각 단계 시작 시: steps[i] = {status: 'running', started_at: now, progress_pct: 0}
3. 단계 진행 시: steps[i].progress_pct = N, progress_message = "..."
4. 단계 완료 시: steps[i] = {status: 'success', finished_at: now, progress_pct: 100, metrics: {...}}
5. 다음 단계로
6. 모든 단계 완료 시: pipeline_run.status='success', finished_at=now
   (실패 시: status='failed', error_message=...)

동시에 OTel:
- 각 단계는 tracing::span (Tempo로 전송)
- 어드민 UI는 자체 진행(JSONB) + Grafana 트레이스 둘 다 표시
```

---

## 11. 검증 기준 (Sub-project 2 완료 판정)

### 11.1 결과물

- [ ] **18 RDS 테이블** + 인덱스 + 제약 모두 정의 (V001__init.sql)
- [ ] **DB role 3개** (writer/reader/audit_archiver) 정의 (V002__db_roles.sql)
- [ ] **Rust 값 객체 15개+** 모두 단위 테스트 (Pnu/Money/Area/BusinessNumber/...)
- [ ] **6 Aggregate Entity** 모든 필드 + 상태 머신 + 도메인 메서드
- [ ] **Repository trait** Aggregate별 (구현체는 sub-project 5)
- [ ] **R2 Reader trait** + R2 디렉토리 구조 정의
- [ ] **Operations 도메인 6개** (admin/verification/review/report/featured/alert)
- [ ] **Pipeline control 도메인** (schedule + run + step JSONB schema)
- [ ] **공유 위젯 데이터 계약** 명시 (Rust types + OpenAPI spec preview)
- [ ] **모든 파일 ≤500줄**

### 11.2 자동 검증

- [ ] `cargo check --workspace` 통과
- [ ] `cargo clippy --workspace -- -D warnings` 통과
- [ ] `cargo test --workspace` 통과 (단위 테스트 90%+ 도메인 커버리지)
- [ ] `cargo deny check` 통과 (라이선스 + 보안)
- [ ] Biome + markdownlint 통과
- [ ] CI 그린 (모든 job)

### 11.3 SSS 15 검증 추가 통과

- [x] (Q4) 의존성 방향 빌드 실패 — `[lints] workspace = true` + dependency-cruiser
- [x] (Q9) 임의 사용자 활동 재구성 가능 — audit_log 기록
- [x] (Q15) 외부 API raw 1년 후 재현 — `gongzzang-raw-archive` 정의

(Q1, Q7, Q10 등은 후속 sub-project 의존)

### 11.4 사용자 검증

- [ ] 사용자가 spec 검토 후 승인
- [ ] 사용자가 결과물 검토 후 승인 (마이그레이션 + 도메인 코드)

---

## 12. 의존성 + 전제

### 12.1 환경

- Rust 1.83 + Cargo workspace (sub-project 1 완료)
- pnpm + Biome (sub-project 1 완료)
- PostgreSQL 17 + PostGIS 3.5 (Docker Compose 로컬, sub-project 8 인프라 전)
- sqlx CLI (개발자 설치)

### 12.2 외부 결정 보류 (이 sub-project에서 안 정함)

- Pulumi RDS 인스턴스 사양 — sub-project 8
- Cloudflare R2 버킷 실제 생성 — sub-project 8
- 데이터 시드 (개발용) — sub-project 5+
- API endpoint URL 패턴 — sub-project 5

---

## 13. 후속 Sub-projects (의존)

```
SP2 (DB + Core 도메인)  ← 현재
 ↓
 ├─▶ SP3 (인증) — User Aggregate + Zitadel JWT 검증
 ├─▶ SP4 (V-World 통합) — Repository 구현 + R2 Reader 구현
 ├─▶ SP5 (첫 API endpoint) — Axum + utoipa
 ├─▶ SP6 (첫 프론트엔드) — admin-web 9 화면 + 공유 위젯
 ├─▶ SP7 (관측성) — OTel + Grafana embed
 ├─▶ SP8 (인프라) — Pulumi RDS + R2 + Role
 └─▶ SP9 (ETL) — 워커가 pipeline_schedule 따름
```

---

## 14. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| Aggregate 경계 모호 (Bookmark가 polymorphic) | 무결성 깨짐 | Listing은 FK, R2 데이터는 polymorphic — 절충 명시 |
| audit_log 폭증 | RDS 디스크 ↑ | 1년 RDS + 6년 R2 IA archive (월 1회 archiver) |
| `pipeline_run.steps` JSONB 크기 폭증 | RDS 디스크 + 쿼리 느림 | step별 metrics는 *짧게*, 큰 데이터는 Loki 링크 |
| R2 sync 동시 실행 (race condition) | 이상 데이터 | Postgres advisory lock + `running_lock_acquired_at` |
| 외부 API raw 보존 7년 비용 | R2 IA 비용 | 압축 (gzip) + 월별 묶음 |
| sub-project 4 (V-World 통합) 시점에 R2 reader trait 변경 | 재작업 | trait를 *최소*로 — 첫 메서드 2-3개만, 확장은 그때 |
| 어드민 운영 데이터 모델이 sub-project 6 UI와 mismatch | 재마이그레이션 | UI 디자인 (sub-project 6 brainstorming) 시점에 V003 마이그레이션 |

---

## 15. 자체 검토 (이 spec)

### Placeholder 스캔
- [ ] 모든 섹션 채워짐 (TBD/TODO 없음)
- 결정 보류는 § 2.3에 명시적

### 내부 일관성
- [ ] § 5 RDS 테이블 18개 = § 4 도메인 분류와 일치
- [ ] ID prefix 모두 glossary와 일치 (usr_, lst_, lph_, ...)
- [ ] DB role (§ 6) = audit_log 정책 (§ 5.3) 일치

### Scope 검증
- [ ] *데이터 모델 + 도메인 코드*에 한정 (UI/API/외부 통합 제외 명시)
- [ ] 후속 sub-project별 책임 명확

### 모호성
- [ ] R2 정적 vs RDS 동적 분류 명확 (§ 4)
- [ ] Aggregate vs 어드민 운영 모델 분리 (§ 5)

---

## 16. 다음 단계

이 spec이 사용자 승인되면:

1. **writing-plans 스킬 호출** — 18 테이블 + 도메인 코드를 Task별 분해 → implementation plan
2. **subagent-driven-development** — Task별 fresh subagent 실행
3. **검증** — § 11 기준 통과 확인

---

## 17. 참조

- ADR: 0001-0011 (sub-project 1)
- 헌법: → @docs/sss-charter.md
- 글로서리: → @docs/glossary.md
- 컨벤션: → @docs/conventions/
- 데이터 소스: → @docs/data-sources/
