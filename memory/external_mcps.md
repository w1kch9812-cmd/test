---
name: 외부 의존 MCP 3종
description: 본 프로젝트가 의존하는 3rd party MCP 서버의 역할·라이선스·사용 규칙
type: project
---

## 의존 MCP

| MCP | 레포 | 라이선스 | 역할 |
|-----|------|---------|------|
| korean-land-mcp | UrbanWatcherKr/korean-land-mcp | MIT | V-World 공간정보 7개 도구 |
| korean-law-mcp | chrisryugj/korean-law-mcp | MIT | 법제처 16개 도구 + verify_citations |
| opendata-mcp | ceami/opendata-mcp | Apache-2.0 | data.go.kr 범용 3개 도구 |

**Why**: 도메인 지식을 처음부터 구현하면 수개월 소요. MIT/Apache 라이선스라 개발자 학습·탐색·reference 자료로 자유 사용 가능.
**How to apply**:
- Claude Code 개발 세션에서 도메인 탐색용으로 활용 (예: "강남구 용도지역 확인")
- 메인 시스템(`apps/web`) 코드에 import 절대 금지
- 도메인 지식(레이어 코드, 법령 매핑 등)은 MCP 소스 코드를 *reference*로 읽고 우리 어댑터로 직접 재구현
- 향후 `apps/ai-assistant/` 추가 시 그 모듈에서만 런타임 의존성 허용

설정: [.mcp.json](../.mcp.json)
상세 문서: [docs/data-sources/](../docs/data-sources/README.md)
