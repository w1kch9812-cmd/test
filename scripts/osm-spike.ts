/**
 * osm-spike.ts
 * OSM data (c) OpenStreetMap contributors, ODbL 1.0 https://www.openstreetmap.org/copyright
 * This script produces STATISTICS ONLY. No raw OSM data is stored or redistributed.
 * Usage: pnpm exec tsx scripts/osm-spike.ts
 *        npx tsx scripts/osm-spike.ts
 */

const SAMPLES = [
  { name: "인천_남동공단", lat: 37.41, lng: 126.72 },
  { name: "안산_반월공단", lat: 37.32, lng: 126.83 },
  { name: "시흥_시화공단", lat: 37.35, lng: 126.75 },
  { name: "청벌_오송", lat: 36.62, lng: 127.34 },
  { name: "천안_직산", lat: 36.88, lng: 127.16 },
  { name: "창원_국가산단", lat: 35.21, lng: 128.64 },
  { name: "울산_미포공단", lat: 35.55, lng: 129.37 },
  { name: "부산_사상공단", lat: 35.15, lng: 128.97 },
  { name: "광주_첨단산단", lat: 35.22, lng: 126.85 },
  { name: "강원_외곽산단", lat: 37.75, lng: 128.89 },
];

const HGV_TAGS = ["hgv", "maxweight", "maxheight", "maxwidth", "maxlength"];
const ROAD_TAGS = ["access", "motor_vehicle", "surface", "width", "lanes"];
const ALL_TAGS = [...HGV_TAGS, ...ROAD_TAGS];

const MAJOR_TYPES = new Set([
  "motorway",
  "trunk",
  "primary",
  "secondary",
  "motorway_link",
  "trunk_link",
  "primary_link",
  "secondary_link",
]);
const LOCAL_TYPES = new Set([
  "tertiary",
  "unclassified",
  "residential",
  "service",
  "tertiary_link",
  "living_street",
  "track",
]);

const writeLine = (message = "") => {
  process.stdout.write(`${message}\n`);
};

function buildQuery(lat, lng) {
  const D = 0.0018;
  const s = (lat - D).toFixed(6),
    n = (lat + D).toFixed(6),
    w = (lng - D).toFixed(6),
    e = (lng + D).toFixed(6);
  return `[out:json][timeout:25];way[${JSON.stringify("highway")}](${s},${w},${n},${e});out tags;`;
}

const OVERPASS_URL = "https://overpass-api.de/api/interpreter";

async function queryOverpass(lat, lng) {
  const qs = buildQuery(lat, lng);
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      const resp = await fetch(OVERPASS_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          "User-Agent": "osm-spike-hgv-research/1.0 (industrial-realestate-study)",
        },
        body: `data=${encodeURIComponent(qs)}`,
        signal: AbortSignal.timeout(30000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return (await resp.json()).elements ?? [];
    } catch (err) {
      if (attempt === 3) throw err;
      await sleep(1000 * attempt);
    }
  }
  return [];
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}
function emptyStats() {
  return { total: 0, tagCounts: Object.fromEntries(ALL_TAGS.map((t) => [t, 0])) };
}

function classifyRoad(tags) {
  const hw = tags.highway ?? "";
  if (MAJOR_TYPES.has(hw)) return "major";
  if (LOCAL_TYPES.has(hw)) return "local";
  return "other";
}

function processSample(elements, name) {
  const major = emptyStats(),
    local = emptyStats(),
    other = emptyStats();
  for (const el of elements) {
    if (el.type !== "way") continue;
    const tags = el.tags ?? {};
    const cat = classifyRoad(tags);
    const stats = cat === "major" ? major : cat === "local" ? local : other;
    stats.total++;
    for (const tag of ALL_TAGS) {
      if (tags[tag] != null && tags[tag] !== "") stats.tagCounts[tag]++;
    }
  }
  return { name, major, local, other, failed: false };
}

function pct(count, total) {
  if (total === 0) return "  N/A";
  return `${String(((count / total) * 100).toFixed(1)).padStart(5)}%`;
}

function printTable(results) {
  writeLine("\n=== Fill-rate per sample (M=major, L=local roads) ===");
  writeLine(["Sample".padEnd(20), "Cat", "N", ...ALL_TAGS].join("\t"));
  for (const r of results) {
    if (r.failed) {
      writeLine([r.name.padEnd(20), "FAILED", "-", ...ALL_TAGS.map(() => "  N/A")].join("\t"));
      continue;
    }
    for (const [label, stats] of [
      ["M", r.major],
      ["L", r.local],
    ]) {
      writeLine(
        [
          r.name.padEnd(20),
          label,
          String(stats.total).padStart(4),
          ...ALL_TAGS.map((t) => pct(stats.tagCounts[t], stats.total)),
        ].join("\t"),
      );
    }
  }
}

function aggregate(results) {
  const major = emptyStats(),
    local = emptyStats();
  let succeeded = 0,
    failed = 0;
  for (const r of results) {
    if (r.failed) {
      failed++;
      continue;
    }
    succeeded++;
    major.total += r.major.total;
    local.total += r.local.total;
    for (const tag of ALL_TAGS) {
      major.tagCounts[tag] += r.major.tagCounts[tag];
      local.tagCounts[tag] += r.local.tagCounts[tag];
    }
  }
  return { major, local, succeeded, failed };
}

function decisionTrigger(s) {
  const hgv = s.total > 0 ? (s.tagCounts.hgv / s.total) * 100 : 0;
  const wt = s.total > 0 ? (s.tagCounts.maxweight / s.total) * 100 : 0;
  const avg = (hgv + wt) / 2;
  if (avg >= 50)
    return `avg hgv+maxweight fill = ${avg.toFixed(1)}% -> ADR-0029: OSM ETL pipeline 추가 진행`;
  if (avg >= 20)
    return `avg hgv+maxweight fill = ${avg.toFixed(1)}% -> ADR-0030: hybrid (OSM + V-World 도로 + broker 입력)`;
  return `avg hgv+maxweight fill = ${avg.toFixed(1)}% -> OSM 직접 활용 보류 - broker 입력 + TMAP for Business API 검토`;
}
async function main() {
  writeLine(`[osm-spike] Start - ${new Date().toISOString()}`);
  writeLine(`[osm-spike] ${SAMPLES.length} sites, Overpass API, ODbL: statistics only`);
  const results = [];
  for (let i = 0; i < SAMPLES.length; i++) {
    const s = SAMPLES[i];
    process.stdout.write(`[osm-spike] [${i + 1}/${SAMPLES.length}] ${s.name} ... `);
    try {
      const els = await queryOverpass(s.lat, s.lng);
      const r = processSample(els, s.name);
      results.push(r);
      writeLine(`${els.length} ways (M:${r.major.total} L:${r.local.total})`);
    } catch (err) {
      writeLine(`ERROR: ${err}`);
      results.push({
        name: s.name,
        major: emptyStats(),
        local: emptyStats(),
        other: emptyStats(),
        failed: true,
      });
    }
    if (i < SAMPLES.length - 1) await sleep(500);
  }
  printTable(results);
  const agg = aggregate(results);
  writeLine(
    `\n=== Sample success/failure ===\n  succeeded: ${agg.succeeded} / failed: ${agg.failed} (failed samples EXCLUDED from aggregate)`,
  );
  if (agg.succeeded === 0) {
    writeLine("[osm-spike] All samples failed — aggregate is meaningless. Abort.");
    process.exit(2);
  }
  writeLine("\n=== Aggregate fill rate (succeeded samples only) ===");
  writeLine(["Cat".padEnd(8), "N", ...ALL_TAGS].join("\t"));
  writeLine(
    [
      "MAJOR".padEnd(8),
      String(agg.major.total).padStart(4),
      ...ALL_TAGS.map((t) => pct(agg.major.tagCounts[t], agg.major.total)),
    ].join("\t"),
  );
  writeLine(
    [
      "LOCAL".padEnd(8),
      String(agg.local.total).padStart(4),
      ...ALL_TAGS.map((t) => pct(agg.local.tagCounts[t], agg.local.total)),
    ].join("\t"),
  );
  writeLine("\n=== Decision trigger (local roads - industrial access priority) ===");
  writeLine(decisionTrigger(agg.local));
  writeLine(`\n[osm-spike] Done - ${new Date().toISOString()}`);
}

main().catch((err) => {
  console.error("[osm-spike] Fatal:", err);
  process.exit(1);
});
