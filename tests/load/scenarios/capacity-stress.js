import { profile, runTags, targetBaseUrl } from "../lib/env.js";
import { safeGet } from "../lib/http.js";

const scenarioName = "capacity-stress";
const defaultListingId = "load-fixture-listing-001";

export const options = {
  scenarios: {
    capacity_stress: {
      executor: "ramping-arrival-rate",
      startRate: 1,
      timeUnit: "1s",
      preAllocatedVUs: Number(__ENV.LOAD_PRE_ALLOCATED_VUS || 100),
      maxVUs: Number(__ENV.LOAD_MAX_VUS || 2000),
      stages: [
        { target: 50, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 100, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 200, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 300, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 400, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 600, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 800, duration: __ENV.LOAD_STRESS_STAGE_DURATION || "2m" },
        { target: 0, duration: "30s" },
      ],
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.05"],
    http_req_duration: ["p(95)<1000", "p(99)<3000"],
  },
};

export function setup() {
  if (__ENV.ALLOW_STRESS !== "true") {
    throw new Error("ALLOW_STRESS=true is required before running capacity-stress");
  }
}

function baseTags(routeGroup, requestKind, priority = "normal") {
  return {
    ...runTags(scenarioName),
    profile: profile(),
    route_group: routeGroup,
    request_kind: requestKind,
    priority,
  };
}

export default function () {
  const baseUrl = targetBaseUrl();
  const listingId = encodeURIComponent(__ENV.LOAD_LISTING_ID || defaultListingId);

  if (Math.random() < 0.7) {
    safeGet(`${baseUrl}/v1/listings`, baseTags("listing", "list", "high"));
    return;
  }

  safeGet(`${baseUrl}/v1/listings/${listingId}`, baseTags("listing", "detail"));
}
