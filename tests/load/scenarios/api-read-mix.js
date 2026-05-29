import { sleep } from "k6";
import { profile, runTags, targetBaseUrl } from "../lib/env.js";
import { safeGet } from "../lib/http.js";

const scenarioName = "api-read-mix";
const defaultListingId = "load-fixture-listing-001";
const defaultPnu = "4113510900100010000";

export const options = {
  scenarios: {
    api_read_mix: {
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
    http_req_duration: ["p(95)<300", "p(99)<1000"],
  },
};

function baseTags(routeGroup, requestKind, priority = "normal") {
  return {
    ...runTags(scenarioName),
    profile: profile(),
    route_group: routeGroup,
    request_kind: requestKind,
    priority,
  };
}

function weightedRequest(baseUrl, selector) {
  const listingId = __ENV.LOAD_LISTING_ID || defaultListingId;
  const pnu = __ENV.LOAD_PNU || defaultPnu;

  if (selector < 0.2) {
    safeGet(`${baseUrl}/health`, baseTags("health", "health", "high"));
    return;
  }

  if (selector < 0.65) {
    safeGet(`${baseUrl}/v1/listings`, baseTags("listing", "list", "high"));
    return;
  }

  if (selector < 0.9) {
    safeGet(
      `${baseUrl}/v1/listings/${encodeURIComponent(listingId)}`,
      baseTags("listing", "detail"),
    );
    return;
  }

  safeGet(
    `${baseUrl}/api/proxy/catalog/v1/parcels/by-pnu/${encodeURIComponent(pnu)}`,
    baseTags("platform_core_catalog", "parcel_by_pnu"),
  );
}

export default function () {
  weightedRequest(targetBaseUrl(), Math.random());
  sleep(Number(__ENV.LOAD_ITERATION_SLEEP_SECONDS || 0));
}
