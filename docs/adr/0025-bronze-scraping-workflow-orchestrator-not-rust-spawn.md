# ADR 0025 — Bronze scraping orchestration = GitHub Actions workflow (Rust 가 Python spawn 안 함)

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Accepted |
| 선행 | [ADR 0022](./0022-bronze-scraping-isolated-python-service.md) |
| Amends | ADR 0022 § "Rust 측 책임" 의 구현 디테일 (격리 원칙은 그대로 유지) |

## 결정

ADR 0022 가 박제한 **격리 원칙** (Python service isolation + 메인 Rust 시스템 의존성 0)
은 그대로 유지. 단, **orchestration pattern** 은 ADR 0022 § 92-99 의 *Rust 가 Python
spawn* 에서 **GitHub Actions workflow 의 phase split** 으로 변경.

```yaml
# .github/workflows/sp9-base-layer-etl.yml
jobs:
  bronze:                                        # Phase 1 — Python only
    runs-on: ubuntu-22.04
    steps:
      - run: python services/scraper-py/dtmk_vworld.py
      # 결과 — R2: bronze/<YYYY-MM>/parcel-dtmk-30563/*.zip

  gold:                                          # Phase 2 — Rust only
    needs: [setup, bronze]
    runs-on: ubuntu-22.04-large
    steps:
      - run: ./target/release/etl-base-layer gold --bronze-prefix ...
      # 입력 — R2: bronze/.../*.zip
      # 출력 — R2: gold/v<N>/<layer>/{z}/{x}/{y}.pbf

  promote:                                       # Phase 3 — Rust only
    needs: [setup, gold]
    runs-on: ubuntu-22.04
    steps:
      - run: ./target/release/etl-base-layer promote --version v<N>
      # 출력 — R2: gold/manifest.json (atomic flip)
```

Rust crate (`services/etl-base-layer/src/bronze/dtmk.rs`) 는 Python 을 spawn 하지
않고, Phase 1 이 적재한 R2 의 zip 들을 *재소비* (list → download → unzip).

## 컨텍스트 — ADR 0022 와의 차이

ADR 0022 § "채택" 의 본문 (line 92-99):

> Rust 측 책임 (ETL orchestrator):
> - subprocess spawn (`tippecanoe` 와 동일 pattern):
>   ```rust
>   build_command(host, "python", &[...])
>   ```
> - stdout JSON parse → manifest 갱신

이 디자인이 검토되었으나 *구현 단계에서* 다음 이유로 workflow-split 으로 전환됨.
ADR 0022 의 *격리 원칙* (Python = isolated service, 메인 Rust crate 의 Python dep 0)
은 양쪽 패턴 모두 만족.

### A — Rust 가 Python spawn (ADR 0022 § 92-99 의 원래 spec)
- Single Rust binary 가 *모든 단계* orchestration — 한 번 실행에 Bronze + Gold + Promote.
- `tippecanoe` / `ogr2ogr` / `tile-join` 와 *완전히 동일한* subprocess pattern.
- 단점:
  - Bronze 가 Python runtime + Scrapling deps 필요 → gold runner 에 Python 환경 동봉
    필수 → `ubuntu-22.04-large` (16 vCPU/64GB) 인스턴스에서 Python venv 까지 설치
  - Bronze 실패 시 Gold 의 12시간 timeout 안에서 retry — runner 비용 폭증
  - bronze 와 gold 의 *작업 특성 다름* (I/O bound vs CPU bound) 인데 동일 runner 강제
  - `--bronze-skip` 같은 *부분 재실행* 패턴 구현 어려움 (한 binary 안 분기 처리)

### B — GitHub Actions workflow phase split (본 ADR 채택, 실제 구현)
- Phase 1 (bronze, ubuntu-22.04) — Python only, 90분 timeout
- Phase 2 (gold, ubuntu-22.04-large) — Rust only, 720분 timeout
- Phase 3 (promote, ubuntu-22.04) — Rust only, 30분 timeout
- Phase 간 통신 = R2 객체 (ground truth)
- 장점:
  - **Resource separation** — Phase 1 은 small runner (90% I/O bound), Phase 2 만 large
    runner (CPU/RAM heavy tippecanoe). Phase 1 에 64GB 안 줘도 됨 → CI 비용 절감
  - **Failure isolation** — Bronze 실패해도 Gold runner instance 안 떠서 비용 0
  - **Restart granularity** — `inputs.bronze_skip=true` 로 Gold 만 재실행 가능 (이미 적재된
    R2 zip 재사용). ADR 0021 의 atomic manifest flip 와 자연 결합.
  - **Concurrency control** — `concurrency.group: sp9-base-layer-etl-${{ github.ref }}` 가
    같은 ref 의 두 번째 ETL 실행 차단. Phase 단위로 간섭 없음.
  - **Job isolation** — Phase 1 의 Python crash 가 Phase 2 의 Rust runner 영향 0
  - **Clearer failure alert** — `notify-failure` job 이 phase 별 result 박제 (`bronze`,
    `gold`, `promote` 각각 success/failure tag)

## SSS 7기둥 매핑

| 기둥 | A (Rust spawn Python) | B (workflow split, 본 결정) |
|---|---|---|
| 일관성 | ✅ — 모든 단계 single binary | ✅ — phase 간 R2 인터페이스 통일 |
| 자동강제 | △ — Python 실패 → Rust 가 retry/abort 결정 | ✅ — workflow runner 가 phase status 강제 |
| 추적성 | ✅ — single tracing context | ✅ — workflow run_id + phase result 박제 |
| 안전성 | △ — runner 한 번 실패 = 모든 phase 영향 | ✅ — phase 격리 |
| 가시성 | △ — 한 거대 log 안 모든 단계 섞임 | ✅ — phase 별 GH Actions UI + Sentry tag |
| SSOT | ✅ | ✅ — R2 가 phase 간 ground truth (둘 다 해당) |
| 명확성 | △ — Rust binary 가 Python venv setup 까지 책임 | ✅ — phase 책임 명확 |

기둥 4개 (자동강제 / 안전성 / 가시성 / 명확성) 가 B 우세. 1개 (일관성) 가 약간 A 우세
(single binary). 종합 SSS = B.

## 격리 원칙 보존

ADR 0022 의 *진짜 결정* 인 격리 원칙은 양쪽 패턴 모두 만족:
- 메인 Rust crate (`crates/`, `services/api`, `services/etl-base-layer`) 의 Python
  의존성 0 ✓
- `services/scraper-py/` 만 Python ✓
- Rust 가 Python 코드 import 0 ✓ (B 는 Rust 가 *Python spawn 도* 안 함 — 더 강한 격리)

## ADR 0022 와의 호환성

ADR 0022 는 **Accepted 상태 유지**. 본 ADR 은 § "Rust 측 책임" 의 *구현 디테일* 만
amendment. 격리 원칙 / 검토 대안 / Scrapling 채택은 그대로 유효.

## 거부 트리거

본 결정 재검토가 필요한 신호:
- ETL 빈도가 daily diff cron 에서 *분 단위* 까지 단축 — workflow startup overhead
  (~30s) 가 병목이 되면 single binary 가 더 빠름
- Python script 수가 5+ 로 증가하여 workflow 가 너무 복잡해짐 — Rust orchestrator 의
  부분 spawn 이 가독성 좋아짐
- GitHub Actions runner 비용이 self-hosted 로 전환 (cost 변수 사라짐)

## 영향

### 수정
- `docs/adr/README.md` — 인덱스 업데이트
- `services/etl-base-layer/src/bronze/dtmk.rs` 의 doc comment 갱신 — 본 ADR 참조

### 신규
- `docs/adr/0025-bronze-scraping-workflow-orchestrator-not-rust-spawn.md` (본 파일)

### 변경 없음
- `services/scraper-py/dtmk_vworld.py` — Phase 1 그대로 동작
- `services/etl-base-layer/src/bronze/dtmk.rs` — 코드 자체는 이미 본 결정 반영
- `.github/workflows/sp9-base-layer-etl.yml` — 이미 phase split 구현

## 참고

- Phase 1: `services/scraper-py/dtmk_vworld.py`
- Phase 2: `services/etl-base-layer/src/main.rs` (`gold` subcommand)
- Phase 3: `services/etl-base-layer/src/gold/promote.rs`
- Workflow: `.github/workflows/sp9-base-layer-etl.yml`
- 원래 ADR: [ADR 0022](./0022-bronze-scraping-isolated-python-service.md)
