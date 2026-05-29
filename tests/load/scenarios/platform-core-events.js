import { check } from "k6";
import exec from "k6/execution";
import http from "k6/http";
import { profile, runTags, targetBaseUrl } from "../lib/env.js";
import { safePostJson, sanitizeTags } from "../lib/http.js";

const scenarioName = "platform-core-events";
const goldPointerEventType = "catalog.industrial_complex.gold_pointer.published.v1";
const poisonEventType = "catalog.unsupported.v1";
const duplicateEventId = "00000000-0000-4000-8000-000000000001";

export const options = {
  scenarios: {
    platform_core_events: {
      executor: "constant-arrival-rate",
      rate: Number(__ENV.LOAD_RPS || 5),
      timeUnit: "1s",
      duration: __ENV.LOAD_DURATION || "2m",
      preAllocatedVUs: Number(__ENV.LOAD_PRE_ALLOCATED_VUS || 10),
      maxVUs: Number(__ENV.LOAD_MAX_VUS || 50),
    },
  },
  thresholds: {
    "http_req_failed{event_case:valid}": ["rate<0.01"],
    "http_req_failed{event_case:duplicate}": ["rate<0.01"],
    "http_req_duration{event_case:valid}": ["p(95)<500", "p(99)<1500"],
    "http_req_duration{event_case:duplicate}": ["p(95)<500", "p(99)<1500"],
  },
};

function baseTags(eventCase, priority = "normal") {
  return {
    ...runTags(scenarioName),
    profile: profile(),
    route_group: "platform_core_events",
    request_kind: "webhook_post",
    event_case: eventCase,
    priority,
  };
}

function uuidForIteration(eventCase) {
  if (eventCase === "duplicate") {
    return duplicateEventId;
  }

  const sequence = exec.scenario.iterationInTest + 1;
  const hex = sequence.toString(16).padStart(12, "0").slice(-12);
  const casePrefix = eventCase === "poison" ? "10000000" : "20000000";
  return `${casePrefix}-0000-4000-8000-${hex}`;
}

function eventBody(eventCase) {
  const eventType = eventCase === "poison" ? poisonEventType : goldPointerEventType;
  const timestamp = new Date().toISOString();

  return {
    event_id: uuidForIteration(eventCase),
    event_type: eventType,
    occurred_at: timestamp,
    scope: "catalog",
    payload: {
      type: eventType,
      schema_version: 1,
      complex_id: "load-industrial-complex-001",
      current_version: `gold-pointer-load-${exec.scenario.iterationInTest}`,
      source_snapshot_id: "industrial-complex-source-load",
      iceberg_snapshot_id: "industrial-complex-iceberg-load",
    },
  };
}

function eventHeaders(body) {
  return {
    "x-platform-core-event-id": body.event_id,
    "x-platform-core-event-type": body.event_type,
    "x-platform-core-outbox-scope": body.scope,
  };
}

function eventCaseForIteration() {
  const iteration = exec.scenario.iterationInTest;
  if (iteration % 10 === 0) {
    return "poison";
  }
  if (iteration % 4 === 0) {
    return "duplicate";
  }
  return "valid";
}

function postPoisonEvent(url, body, tags, headers) {
  const response = http.post(url, JSON.stringify(body), {
    headers: { "Content-Type": "application/json", ...headers },
    tags: sanitizeTags(tags),
  });
  check(response, {
    "poison event is rejected": (r) => r.status === 400,
  });
  return response;
}

export default function () {
  const baseUrl = targetBaseUrl();
  const currentCase = eventCaseForIteration();
  const body = eventBody(currentCase);
  const url = `${baseUrl}/platform-core/events`;
  const tags = baseTags(currentCase, currentCase === "valid" ? "high" : "normal");
  const headers = eventHeaders(body);

  if (currentCase === "poison") {
    postPoisonEvent(url, body, tags, headers);
    return;
  }

  safePostJson(url, body, tags, headers);
}
