# crates/domain/insights

Insights Bounded Context — 사용자 가치·분석 도메인.

## Aggregates
- **Bookmark** — 매수자가 저장한 매물·회사·필지
- **SearchHistory** — 검색 이력 (PIPA 마스킹)
- **AnalysisReport** — 사용자가 생성·저장한 분석 리포트
- **Notification** — 즐겨찾기 변경, 시세 변동 등

## 의존
- `crates/domain/shared-kernel`
- 다른 BC의 ID만 참조 (event 구독)

## 정책
- 사용자 행동 데이터 = PIPA 준수 (가입 시 동의 + 익명화)
- AnalysisReport 본문 = 사용자가 작성한 한국어 또는 데이터 시각화 (LLM 생성 X)
- Notification 채널 = 인앱 + 이메일 + FCM (Phase 2+)
- 검색 이력 보존: 90일 (그 후 가명화 또는 삭제)
