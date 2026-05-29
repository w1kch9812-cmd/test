# ADR 0024 — ETL Cancel Protocol = 즉시 abort + L3 staging 보호

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Accepted |
| 선행 | [ADR 0021](./0021-static-vector-tile-decomposition.md) (X9 PMTiles 분해), [ADR 0023](./0023-audit-2026-05-08-hardening.md) (Round 1 audit) |
| 관련 | [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (Medallion + L3 atomicity) |

## 결정

`services/etl-base-layer` 의 cancel protocol = **즉시 abort + ExitCode 130 (SIGINT)**.
명시적 state machine 또는 graceful checkpoint resume 은 **채택하지 않음**.

```rust
// services/etl-base-layer/src/main.rs:86-88
let task = match subcommand {
    "bronze" | "" => tokio::spawn(run_bronze()),
    "gold" => tokio::spawn(run_gold(args[1..].to_vec())),
    "promote" => tokio::spawn(run_promote_cli(args[1..].to_vec())),
    ...
};
tokio::select! {
    biased;
    result = task => { ... },
    () = shutdown_signal() => {
        warn!("shutdown signal received — aborting (L3 staging spec 가 prod 보호)");
        ExitCode::from(130)
    }
}
```

## 컨텍스트

ETL pipeline 단계는 길다 — tippecanoe 빌드 자체가 평균 2-4시간 (전국 1.4억
parcel polygon, ubuntu-22.04-large 기준). 사용자의 `Ctrl+C` 또는 GH Actions 의
SIGTERM (job timeout) 시 중간 결과를 어떻게 보존할지 결정 필요.

대안:

### A — Immediate abort (본 결정)
- 현재 step 즉시 중단, 부분 산출물 cleanup 책임은 *없음*
- L3 staging spec (`gold/staging/<version>/<layer>.spec.json`) 가 *atomic publish*
  보호 — 즉시 abort 라도 prod manifest 변경 0
- 다음 run = 처음부터 재시작

### B — Graceful state machine (거부)
- Bronze → Gold (ogr2ogr → tippecanoe → decompose → R2 batch upload) → Verify →
  Promote 단계마다 checkpoint
- abort 시 last-completed checkpoint 박제, 다음 run 이 그 지점부터 resume
- 구현 비용:
  - checkpoint state schema (R2 또는 로컬 file)
  - 각 단계 idempotency 검증 (이미 일부 있음 — dtmk fetch 의 size diff, decompose 의
    output_dir 검사 등)
  - tippecanoe 자체 resume 불가 — 1.4억 polygon 빌드 부분 결과 활용 X. tippecanoe
    중단 시점에서 재개해야 *진짜* 가치
  - ETL state 의 ground truth 는 결국 R2 (staging spec / manifest) — 별도 state file
    은 R2 와의 drift 위험

## 정당화

본 ADR 이 박제하는 *왜 즉시 abort 가 SSS 인가*:

1. **빈도 — 월 1회**. ETL 은 GitHub Actions cron (매월 1일 03:00 KST). 사용자
   `Ctrl+C` 는 dev local 한정 (production 은 cron + workflow_dispatch 만). dev local
   smoke 의 abort cost 는 trivial.

2. **L3 atomicity 가 partial state 차단**. promote subcommand 가 모든 layer 의
   staging spec 검증 후에만 manifest atomic flip. 즉시 abort = staging buffer 만
   남고 prod manifest 변경 0. 클라이언트는 이전 manifest 그대로 fetch — graceful
   degrade.

3. **tippecanoe 가 resume 불가능**. 단계 B (state machine) 의 진짜 가치는 tippecanoe
   resume — 그게 안 되면 12h timeout 의 11h 째 abort 라도 다음 run 이 처음부터.
   B 채택 시 cost (구현 + 유지) > value (Bronze 단계만 skip 가능).

4. **L8 — ExitCode 130 명시**. SIGINT (128 + 2) bash convention. CI runner 가
   "사용자 중단" 으로 인지 (워크플로 자체 실패 alert 와 별개로 분류).

5. **R2 = ground truth**. ETL 이 가진 모든 *영구* 상태는 R2 객체로 박제. local 디스크
   state (var/gold/, var/dtmk-work/) 는 idempotent helper 들이 다음 run 에서 자동 reuse:
   - dtmk fetch — 같은 size 면 다운 skip, 추출 dir 에 .shp 있으면 unzip skip
   - ogr2ogr — 출력 .geojson 비어있지 않으면 skip
   - tippecanoe — `--force` flag 로 덮어쓰기 (단계 A 의 한계 — 단 빠른 step)
   - decompose — output_dir 비어있어야 진행 (호출자가 정리)
   - R2 batch upload — flat tile 은 immutable URL, 같은 key 재 PUT 도 idempotent

## SSS 7기둥 매핑

| 기둥 | 즉시 abort (본 결정) | State machine |
|---|---|---|
| 일관성 | ✅ — 모든 subcommand 동일 path | △ — checkpoint 형식이 단계별 다름 |
| 자동강제 | ✅ — `select!` 가 시그널 가로챔, OS-level | △ — checkpoint write 누락 시 silent drift |
| 추적성 | ✅ — exit 130 + tracing warn 박제 | ✅ — checkpoint 가 last-step 박제 |
| 안전성 | ✅ — partial state 가 prod 닿지 않음 (L3 atomicity) | △ — checkpoint state 자체가 새 실패 모드 |
| 가시성 | ✅ — Sentry / CI runner 즉시 알림 | △ — resume 흐름이 *왜* 불명확 |
| SSOT | ✅ — R2 = ground truth, 별도 state 0 | ❌ — checkpoint state vs R2 drift 위험 |
| 명확성 | ✅ — "끊으면 처음부터" — 사용자가 추측 0 | △ — resume 동작 박제 학습 필요 |

본 결정은 *L3 staging atomicity* 가 partial state risk 를 cover 하기 때문에 가능.
Medallion 패턴 자체가 cancel safety 의 1차 방어선 — state machine 은 *그 위에*
optimization 이지 safety 아님.

## 거부 트리거

본 결정 재검토가 필요한 신호:

- ETL 빈도가 월 1회 → 일 1회 또는 그 이상으로 변경
- tippecanoe 가 incremental resume 지원 추가 (felt fork 로드맵에는 미포함, 2026-05 기준)
- dev 로컬 사용 패턴 변경 — abort cost 가 빈번한 friction 으로 박제됨
- L3 staging atomicity 자체가 깨지는 새로운 단계 추가

## 영향

### 수정
- `services/etl-base-layer/src/main.rs` — `tokio::select!` 위에 본 ADR 참조 주석 추가
- `docs/adr/README.md` — 인덱스 업데이트

### 신규
- `docs/adr/0024-etl-cancel-protocol-immediate-abort.md` (본 파일)

## 참고

- bash signal convention (128 + signum): <https://tldp.org/LDP/abs/html/exitcodes.html>
- Medallion atomicity (Bronze → Silver → Gold): [ADR 0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md)
- L3 atomicity 구현: `services/etl-base-layer/src/gold/promote.rs` — staging spec → manifest atomic flip
