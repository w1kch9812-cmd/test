import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

/**
 * utoipa (services/api) 가 출력한 OpenAPI spec → TypeScript types.
 *
 * 사용:
 *   1) services/api 가 utoipa 로 OpenAPI spec 출력 (예: services/api/openapi.json)
 *   2) `pnpm --filter @gongzzang/api-types generate` 실행
 *   3) packages/api-types/generated/schema.ts 에 types 작성
 *
 * 본 sub-project (T3) 는 스크립트만. utoipa 미통합 시 placeholder 유지.
 */

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const OPENAPI_PATH = resolve(__dirname, "../../../services/api/openapi.json");
const OUTPUT_PATH = resolve(__dirname, "../generated/schema.ts");

async function main(): Promise<void> {
  let openapiContent: string;
  try {
    openapiContent = await readFile(OPENAPI_PATH, "utf-8");
  } catch {
    console.warn(`[api-types] OpenAPI spec not found at ${OPENAPI_PATH}. Keeping placeholder.`);
    return;
  }

  const types = astToString(await openapiTS(JSON.parse(openapiContent)));
  await writeFile(OUTPUT_PATH, types, "utf-8");
  console.info(`[api-types] Generated TS types at ${OUTPUT_PATH}`);
}

main().catch((err) => {
  console.error("[api-types] Generation failed:", err);
  process.exit(1);
});
