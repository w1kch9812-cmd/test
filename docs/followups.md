# Follow-ups (open work)

작업 진행 중 발견된 이슈 / 다음 작업 / decision-pending 항목 SSOT.
완료된 항목은 git history (`git log --grep`) 또는 ADR 로 이전.

마지막 업데이트: 2026-05-06

---

## P0 — Blocking real product traffic

### 1. Auth user provisioning 실패 (Zitadel access_token 에 email 없음)

**증상**: Frontend 로그인 후 `/api/proxy/listings` → `/listings` (백엔드) 호출 시
`AUTH_USER_PROVISION_FAILED` (500). Zitadel access_token JWT 가 `email`/
`preferred_username` claim 을 포함하지 않아 [`provision_new_user`](../crates/auth/src/middleware.rs#L119)
가 새 user 생성 못 함.

**원인**: Zitadel 기본 동작 — access_token 은 sub/aud/exp 만, user info 는
`/oauth/v2/userinfo` 별도 endpoint. scope 에 `email profile` 줘도 자동 안 들어감.

**처리 옵션 (사용자 결정 필요)**:

- **A. Zitadel 프로젝트 설정**: admin 콘솔에서 application token settings 의
  "User info in access token" 토글. 1분, production-친화.
- **B. Backend userinfo fallback**: JWT 에 email 없으면 Zitadel `/oauth/v2/userinfo`
  호출해서 채움. 30분 Rust 작업 + 테스트. SSS-grade.
- **C. Backend dev placeholder**: email 없으면 `<sub>@zitadel.local`. 5분, dev-only.

**현재 상태**: 백엔드 직접 호출 (DEV.* 토큰 with `AUTH_DEV_MODE=true`) 은 200 OK 확인.
실제 Zitadel JWT 로는 위 fix 필요.

**예상 처리 시점**: SP6-iii (매물 상세) 또는 SP6-data-vworld 시작 전.

---

## P1 — Should fix before next sub-project

### 2. SP6-ii-gl: Naver gl WebGL 커스텀 레이어 (deck.gl + mapbox-gl)

**현재 상태**: `gl: true` 로 Naver WebGL 백엔드 활성화 + `_mapbox` 인스턴스 부착
확인 ([`listing-map.tsx`](../apps/web/components/listings/listing-map.tsx#L30)).
하지만 그 위에 아무 layer 도 안 그림.

**SSS 격차** (vs `C:\Users\User\Desktop\gongzzang\gongzzang\apps\gongzzang-design-lab`):

- ✗ `mapbox-gl ^3.18.1` 의존성 없음
- ✗ 매물 마커가 `naver.maps.Marker` DOM 핀 (천 개+ 가면 스크롤 끊김) — WebGL
  custom layer (mapbox circle/symbol) 로 옮겨야 60fps
- ✗ 클러스터링 자리 (`submodules=clustering` 로드만 하고 사용 X)
- ✗ Per-layer 에러 격리 (MapLayerErrorBoundary)
- ✗ 호버 highlight + Canvas 툴팁
- ✗ 3D tilt + fog (선택 — 산업단지 polygon 있을 때 필요)
- ✗ Debug 패널 (FPS / memory / layer 토글)

**예상 작업**: 3-4일 (별도 sub-project 로 brainstorming → spec → plan → 실행).

---

### 3. 디자인 시스템 — Claude.com brand 임시 차용 → 공짱 brand 로 교체

**현재 상태**: Claude.com 의 cream/coral/dark-navy trinity + spec 의 typography
tokens 그대로 차용 (Pretendard Variable 만 한글용으로 sub).
[packages/ui/tokens/colors.css](../packages/ui/tokens/colors.css) 상단 주석 참조.

**격차**:

- brand voice 는 anthropic.com 의 것이지 공짱의 것이 아님
- 산업용 부동산 다움 0
- 향후 디자이너와 brand identity 결정 → token 값만 갈아끼우면 component 코드는
  0줄 변경 (token 이름 표준이라 미래 작업 손실 없음)

**예상 처리 시점**: 디자이너 합류 또는 brand identity 워크샵 후.

---

## P2 — Tech debt

### 4. Pretendard dynamic-subset 92 파일 self-host

**상태**: `apps/web/public/fonts/woff2-dynamic-subset/` 에 92 파일 (3.1 MB) 들어있음.
브라우저는 `unicode-range` 매칭으로 실제 사용 글자 ~5-10 파일만 다운로드 (150-300 KB).

**개선 가능**:

- CDN proxy 또는 Cloudflare R2 로 넘겨서 main bundle 안 부풀림
- 빌드 시 사용 글자 분석해서 미리 subset 미리지정
- 하지만 현재 dev 환경에서는 차고 넘침 (font 587 KB / total 3.18 MB)

**예상 처리 시점**: production 배포 시 검토.

---

### 5. Frontend `gongzzang_2` Postgres 호스트 포트 5500 (Windows reserved range)

**원인**: Windows 가 5360-5459 + 일부 다른 영역을 Hyper-V dynamic port range 로
예약. 5432 / 5433 / 5434 모두 bind 실패.

**현재 처리**: `infrastructure/docker/docker-compose.yml` 에서 `5500:5432`,
`.env` 의 `DATABASE_URL=...@localhost:5500/...`.

**향후**: Linux/macOS 개발자한테는 5432 가 자연스러움. WSL 또는 cross-platform
조건부 포트 설정 검토 (production 은 무관).

---

### 6. Naver gl tile cache 제한 미설정

**현재**: 기본 200-500 MB. 4GB 디바이스에서 메모리 압박.

**개선**: `(map as any)._mapbox.setMaxTileCacheSize(<디바이스메모리비례>)` 호출
([reference 의 useMapboxGLInit:67-69](C:/Users/User/Desktop/gongzzang/gongzzang/apps/gongzzang-design-lab/components/map/naver/hooks/useMapboxGLInit.ts#L67))
패턴.

**예상 처리 시점**: SP6-ii-gl (item 2) 와 묶어서 처리.

---

## Done (참고용 — 이번 commit 으로 처리됨)

- ✓ Naver gl: true 활성화 (head sync `<script>` + WebGL context recovery)
- ✓ CSP `strict-dynamic` 제거 (head sync script 차단 원인) + dev `script-src http:`
- ✓ React Strict Mode 비활성화 (Naver gl 이중 렌더 방지)
- ✓ 매물 종류 노출 3개 로 축소 (factory / warehouse / industrial_land "토지" label)
- ✓ Pretendard Variable dynamic-subset (4 weight self-host 9.3 MB → 0.6 MB)
- ✓ Claude.com 디자인 시스템 적용 (token + primitive + 화면)
- ✓ raw `<select>` → shadcn `<Select>` primitive
- ✓ 누락 primitive 추가 (Select / Skeleton / Separator / Badge)
- ✓ Postgres 5500 포트 + 마이그레이션 적용
- ✓ API 8080 구동 (DEV 토큰 200 OK 확인)
