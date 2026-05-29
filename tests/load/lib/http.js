import { check } from "k6";
import http from "k6/http";

const allowedTagKeys = new Set([
  "scenario",
  "environment",
  "git_sha",
  "profile",
  "route_group",
  "request_kind",
  "cache_state",
  "event_case",
  "priority",
]);

const maxTagValueLength = 80;

if (typeof http.setResponseCallback === "function" && typeof http.expectedStatuses === "function") {
  http.setResponseCallback(http.expectedStatuses({ min: 200, max: 299 }, 409, 429));
}

function sanitizeTagValue(value) {
  const sanitized = String(value)
    .replace(/[^A-Za-z0-9_.:-]/g, "_")
    .slice(0, maxTagValueLength);
  return sanitized || "unknown";
}

export function sanitizeTags(tags = {}) {
  const safeTags = {};
  for (const [key, value] of Object.entries(tags)) {
    if (!allowedTagKeys.has(key) || value === undefined || value === null) {
      continue;
    }
    safeTags[key] = sanitizeTagValue(value);
  }
  return safeTags;
}

export function safeGet(url, tags, headers = {}) {
  const response = http.get(url, { headers, tags: sanitizeTags(tags) });
  check(response, {
    "status is 2xx or controlled 4xx": (r) =>
      (r.status >= 200 && r.status < 300) || r.status === 409 || r.status === 429,
  });
  return response;
}

export function safePostJson(url, body, tags, headers = {}) {
  const response = http.post(url, JSON.stringify(body), {
    headers: { "Content-Type": "application/json", ...headers },
    tags: sanitizeTags(tags),
  });
  check(response, {
    "status is accepted or controlled rejection": (r) =>
      (r.status >= 200 && r.status < 300) || r.status === 409 || r.status === 429,
  });
  return response;
}
