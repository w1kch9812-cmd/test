import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

/**
 * Generate TypeScript API contract types from the Rust API OpenAPI document.
 *
 * The generator must fail when `services/api/openapi.json` is absent. Keeping a
 * hand-written placeholder would make the frontend believe an API contract was
 * generated when no Rust source-of-truth exists.
 */

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const OPENAPI_PATH = resolve(__dirname, "../../../services/api/openapi.json");
const OUTPUT_PATH = resolve(__dirname, "../generated/schema.ts");

async function main(): Promise<void> {
  let openapiContent: string;
  try {
    openapiContent = await readFile(OPENAPI_PATH, "utf-8");
  } catch (error) {
    throw new Error(`OpenAPI spec not found at ${OPENAPI_PATH}`, { cause: error });
  }

  const types = astToString(await openapiTS(JSON.parse(openapiContent)));
  await writeFile(OUTPUT_PATH, types, "utf-8");
  console.info(`[api-types] Generated TS types at ${OUTPUT_PATH}`);
}

main().catch((err) => {
  console.error("[api-types] Generation failed:", err);
  process.exit(1);
});
