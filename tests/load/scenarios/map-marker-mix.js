import { sleep } from "k6";
import { profile, runTags, targetBaseUrl } from "../lib/env.js";
import { safeGet, safePostJson } from "../lib/http.js";

const scenarioName = "map-marker-mix";
const defaultFilterHash = "all-active-v1";
const defaultMaskBaseVersion = "1";
const defaultTile = { z: "14", x: "8780", y: "6345" };
const defaultMissTile = { z: "14", x: "8781", y: "6345" };

export const options = {
  scenarios: {
    map_marker_mix: {
      executor: "constant-arrival-rate",
      rate: Number(__ENV.LOAD_RPS || 10),
      timeUnit: "1s",
      duration: __ENV.LOAD_DURATION || "5m",
      preAllocatedVUs: Number(__ENV.LOAD_PRE_ALLOCATED_VUS || 20),
      maxVUs: Number(__ENV.LOAD_MAX_VUS || 100),
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
    "http_req_duration{cache_state:hit}": ["p(95)<100"],
    "http_req_duration{cache_state:miss}": ["p(95)<500"],
    http_req_duration: ["p(99)<1500"],
  },
};

function fixtureTile(cacheState) {
  const prefix = cacheState === "miss" ? "LOAD_MARKER_MISS" : "LOAD_MARKER_HIT";
  const fallback = cacheState === "miss" ? defaultMissTile : defaultTile;

  return {
    z: __ENV[`${prefix}_Z`] || __ENV.LOAD_MARKER_Z || fallback.z,
    x: __ENV[`${prefix}_X`] || __ENV.LOAD_MARKER_X || fallback.x,
    y: __ENV[`${prefix}_Y`] || __ENV.LOAD_MARKER_Y || fallback.y,
  };
}

function baseTags(routeGroup, requestKind, cacheState, priority = "normal") {
  return {
    ...runTags(scenarioName),
    profile: profile(),
    route_group: routeGroup,
    request_kind: requestKind,
    cache_state: cacheState,
    priority,
  };
}

function filterHash(cacheState) {
  if (cacheState === "miss") {
    return __ENV.LOAD_FILTER_HASH_MISS || __ENV.LOAD_FILTER_HASH || defaultFilterHash;
  }

  return __ENV.LOAD_FILTER_HASH || defaultFilterHash;
}

function markerFilterPayload(cacheState) {
  return {
    types: splitList(__ENV.LOAD_MARKER_TYPES),
    transactions: splitList(__ENV.LOAD_MARKER_TRANSACTIONS),
    min_area_m2: numberOrNull(__ENV.LOAD_MARKER_MIN_AREA_M2),
    max_area_m2: numberOrNull(__ENV.LOAD_MARKER_MAX_AREA_M2),
    min_price_krw:
      cacheState === "miss"
        ? numberOrNull(__ENV.LOAD_MARKER_MISS_MIN_PRICE_KRW)
        : numberOrNull(__ENV.LOAD_MARKER_MIN_PRICE_KRW),
    max_price_krw: numberOrNull(__ENV.LOAD_MARKER_MAX_PRICE_KRW),
  };
}

function splitList(value) {
  if (!value) {
    return [];
  }

  return value
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function numberOrNull(value) {
  if (value === undefined || value === null || String(value).trim() === "") {
    return null;
  }

  return Number(value);
}

function requestMarker(baseUrl, selector, cacheState) {
  const tile = fixtureTile(cacheState);
  const hash = encodeURIComponent(filterHash(cacheState));
  const baseVersion = encodeURIComponent(__ENV.LOAD_MASK_BASE_VERSION || defaultMaskBaseVersion);

  if (selector < 0.55) {
    safeGet(
      `${baseUrl}/api/proxy/map/v1/marker-tiles/listing/${encodeURIComponent(tile.z)}/${encodeURIComponent(tile.x)}/${encodeURIComponent(tile.y)}.pbf?filter_hash=${hash}`,
      baseTags("listing_marker", "tile_pbf", cacheState, "high"),
    );
    return;
  }

  if (selector < 0.75) {
    safeGet(
      `${baseUrl}/api/proxy/map/v1/marker-counts/listing?filter_hash=${hash}`,
      baseTags("listing_marker", "counts", cacheState),
    );
    return;
  }

  if (selector < 0.9) {
    safePostJson(
      `${baseUrl}/api/proxy/map/v1/marker-filters/listing`,
      markerFilterPayload(cacheState),
      baseTags("listing_marker", "filters", cacheState),
    );
    return;
  }

  safeGet(
    `${baseUrl}/api/proxy/map/v1/marker-masks/listing/${encodeURIComponent(tile.z)}/${encodeURIComponent(tile.x)}/${encodeURIComponent(tile.y)}?filter_hash=${hash}&base_version=${baseVersion}`,
    baseTags("listing_marker", "masks", cacheState),
  );
}

export default function () {
  const cacheState = Math.random() < 0.8 ? "hit" : "miss";
  requestMarker(targetBaseUrl(), Math.random(), cacheState);
  sleep(Number(__ENV.LOAD_ITERATION_SLEEP_SECONDS || 0));
}
