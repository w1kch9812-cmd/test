/**
 * Placeholder — utoipa OpenAPI 통합 시점에 자동 생성.
 *
 * `pnpm --filter @gongzzang/api-types generate` 실행으로 갱신.
 * 본 sub-project (SP6-foundation T3) 는 minimal stub.
 */

export interface paths {
  "/healthz": {
    get: {
      responses: {
        200: {
          content: {
            "text/plain": string;
          };
        };
      };
    };
  };
}

export type components = Record<string, never>;
