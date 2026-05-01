# packages/api-client

Rust API → TypeScript SDK 자동 생성.

## 자동 생성 흐름
```
Rust (utoipa) → openapi.json → openapi-typescript → types.ts
                                                  → openapi-fetch → client
```

## 빌드 단계 (sub-project 5+)
1. `cargo run --bin openapi-export` → `openapi.json` 생성
2. `pnpm openapi-typescript openapi.json -o src/types.ts` 자동 실행 (Turbo task)
3. `pnpm openapi-fetch` 클라이언트 wrapper

## 정책
- `src/types.ts` = **자동 생성만**, 수동 편집 차단 (CI lint)
- `src/index.ts` = client 인스턴스 export + 인증 헤더 헬퍼
- 모든 다른 워크스페이스가 이 패키지 import (직접 fetch X)
- OpenAPI 변경 = `pnpm turbo build` 시 자동 갱신

## 사용 예시
```ts
import { rustApi } from "@gongzzang/api-client";
const { data, error } = await rustApi.GET("/v1/listings/{id}", {
  params: { path: { id: "lst_..." } },
});
```

→ ADR-0006, → @docs/conventions/typescript.md
