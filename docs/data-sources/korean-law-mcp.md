# korean-law-mcp (에이전트 경로)

- 소스: https://github.com/chrisryugj/korean-law-mcp
- 라이선스: MIT
- 역할: **법제처 41개 API → 16개 도구**, 법령·판례·조례·조약 검색 + 환각 방지
- 프로덕션 사용 금지 — 프로덕션은 `open.law.go.kr` 직접 호출

## 원격 배포

- `korean-law-mcp.fly.dev` (공식 배포)
- 로컬 실행도 가능 (`npx korean-law-mcp`)

## 핵심 도구

### Core (3)
| 도구 | 기능 |
|------|------|
| `search_laws` | 법령 검색 |
| `get_law_text` | 전문 조회 |
| `get_annex` | 별표/서식 추출 |

### Unified (2) — 17개 판례 도메인 통합
| 도구 | 기능 |
|------|------|
| `search_decisions` | 판례/헌재/조세심판/공정위 등 통합 검색 |
| `get_decision_text` | 전문 조회 |

### Chain (8) — 복합 워크플로우
| 도구 | 기능 |
|------|------|
| `full_legal_analysis` | 종합 법률 분석 |
| `law_system_map` | 법체계 매핑 |
| `admin_action_basis` | 행정처분 근거 |
| `dispute_prep` | 분쟁 대응 준비 |
| `amendment_tracking` | 개정 이력 |
| `ordinance_comparison` | 조례 비교 |
| `document_review` | 문서 검토 |
| (1 더) | |

### Meta (2)
| 도구 | 기능 |
|------|------|
| `discover_tools` | 자연어 도구 탐색 |
| `execute` | 범용 프록시 |

## ⭐ 환각 방지: `verify_citations`

LLM이 생성한 법령 인용을 실시간 팩트체크.

```
입력: "국토계획법 제76조제5항에 따라 우선위임된다."
출력:
  - "국토의 계획 및 이용에 관한 법률 제76조제5항": ✓ (존재)
```

**본 프로젝트 규칙**: 법령 인용을 포함한 모든 산출물 작성 전 의무 실행 — [AGENTS.md §4](../../AGENTS.md).

## 문서 포맷 지원

- HWPX / HWP (한글), PDF, XLSX, DOCX (kordoc 엔진)

## 캐싱

- 검색: 1시간
- 문서 전문: 24시간

## 인증

```
KOREAN_LAW_API_KEY=...
```

발급: https://open.law.go.kr/LSO/openApi/guideList.do

## 지원 클라이언트

Claude Code, Claude Desktop, Claude.ai (Pro/Max/Team), Cursor, Windsurf, Zed.
