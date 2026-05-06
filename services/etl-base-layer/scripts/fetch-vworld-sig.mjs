#!/usr/bin/env node
/**
 * fetch-vworld-sig.mjs — V-World API 에서 한 SIG (시군구) 의 필지 GeoJSON 을 받아
 * `<output>` 에 단일 FeatureCollection 으로 저장.
 *
 * 사용:
 *   node services/etl-base-layer/scripts/fetch-vworld-sig.mjs \
 *     --sig 11680 --output ./var/sample/gangnam.geojson [--max-pages 5]
 *
 * 환경변수 (.env 에서 직접 읽음):
 *   VWORLD_API_KEY  (필수)
 *   VWORLD_DOMAIN   (필수, Origin/Referer 헤더에 사용)
 *
 * SP9 T3b.2 의 로컬 smoke 전용. Production 빌드는 SHP from 공공데이터포털 사용.
 */

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// __dirname polyfill for ESM
const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..", "..", "..");

// ===== arg parsing =====
const args = process.argv.slice(2);
function getArg(name, fallback) {
  const i = args.indexOf(name);
  if (i === -1 || i === args.length - 1) return fallback;
  return args[i + 1];
}
const SIG = getArg("--sig", "11680"); // 강남구 default
const OUTPUT = getArg("--output", resolve(REPO_ROOT, "var/sample/gangnam.geojson"));
const MAX_PAGES = Number.parseInt(getArg("--max-pages", "5"), 10);
const PAGE_SIZE = 1000;

// ===== .env loader (KEY=VALUE per line) =====
function loadEnv() {
  const envPath = resolve(REPO_ROOT, ".env");
  const raw = readFileSync(envPath, "utf8");
  const out = {};
  for (const line of raw.split(/\r?\n/)) {
    const m = line.match(/^([A-Z0-9_]+)=(.*)$/);
    if (m) out[m[1]] = m[2].trim();
  }
  return out;
}
const env = loadEnv();
const API_KEY = env.VWORLD_API_KEY;
const DOMAIN = env.VWORLD_DOMAIN;
if (!API_KEY || !DOMAIN) {
  console.error("VWORLD_API_KEY / VWORLD_DOMAIN missing in .env");
  process.exit(2);
}

// ===== fetch one page =====
async function fetchPage(page) {
  const url = new URL("https://api.vworld.kr/req/data");
  url.searchParams.set("key", API_KEY);
  url.searchParams.set("service", "data");
  url.searchParams.set("version", "2.0");
  url.searchParams.set("request", "GetFeature");
  url.searchParams.set("data", "LP_PA_CBND_BUBUN");
  url.searchParams.set("attrFilter", `pnu:like:${SIG}`);
  url.searchParams.set("crs", "EPSG:4326");
  url.searchParams.set("format", "json");
  url.searchParams.set("geometry", "true");
  url.searchParams.set("attribute", "true");
  url.searchParams.set("size", String(PAGE_SIZE));
  url.searchParams.set("page", String(page));

  const res = await fetch(url, {
    headers: { Referer: DOMAIN, Origin: DOMAIN },
    signal: AbortSignal.timeout(60_000),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status} on page ${page}`);
  const json = await res.json();
  if (json?.response?.status === "NOT_FOUND") return null;
  if (json?.response?.status === "ERROR") {
    throw new Error(`V-World ERROR: ${JSON.stringify(json.response).slice(0, 300)}`);
  }
  return json;
}

// ===== main =====
async function main() {
  console.log(`Fetching SIG=${SIG} (max ${MAX_PAGES} pages of ${PAGE_SIZE})`);

  // probe page 1 to get total
  const probe = await fetchPage(1);
  if (!probe) {
    console.error(`SIG ${SIG} not found`);
    process.exit(3);
  }
  const total = Number.parseInt(probe?.response?.page?.total ?? 0, 10);
  const totalPages = Math.min(Math.ceil(total / PAGE_SIZE), MAX_PAGES);
  console.log(`  total=${total.toLocaleString()} features, fetching ${totalPages} page(s)`);

  const allFeatures = [];
  // re-use probe's features as page 1 result
  const features1 = probe?.response?.result?.featureCollection?.features ?? [];
  for (const f of features1) {
    allFeatures.push({
      type: "Feature",
      properties: { pnu: f?.properties?.pnu },
      geometry: f.geometry,
    });
  }

  for (let p = 2; p <= totalPages; p++) {
    process.stdout.write(`  page ${p}/${totalPages} ...\r`);
    const json = await fetchPage(p);
    const features = json?.response?.result?.featureCollection?.features ?? [];
    for (const f of features) {
      allFeatures.push({
        type: "Feature",
        properties: { pnu: f?.properties?.pnu },
        geometry: f.geometry,
      });
    }
  }
  console.log(`\n  fetched ${allFeatures.length.toLocaleString()} features`);

  mkdirSync(dirname(OUTPUT), { recursive: true });
  const fc = { type: "FeatureCollection", features: allFeatures };
  writeFileSync(OUTPUT, JSON.stringify(fc));
  console.log(`  wrote ${OUTPUT}`);
}

await main();
