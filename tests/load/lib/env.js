export function requireEnv(name) {
  const value = __ENV[name];
  if (!value || value.trim() === "") {
    throw new Error(`${name} is required`);
  }
  return value;
}

function hostnameFromUrl(url) {
  if (/\s/.test(url)) {
    throw new Error("TARGET_BASE_URL must be a valid URL");
  }

  const match = url.match(/^[a-z][a-z0-9+.-]*:\/\/([^/?#]+)/i);
  if (!match) {
    throw new Error("TARGET_BASE_URL must be a valid URL");
  }

  const authority = match[1];
  const hostWithPort = authority.slice(authority.lastIndexOf("@") + 1);
  if (hostWithPort.startsWith("[")) {
    const end = hostWithPort.indexOf("]");
    if (end === -1) {
      throw new Error("TARGET_BASE_URL must be a valid URL");
    }
    return hostWithPort.slice(1, end).toLowerCase().replace(/\.+$/, "");
  }

  const hostname = hostWithPort.split(":")[0].toLowerCase().replace(/\.+$/, "");
  if (!hostname) {
    throw new Error("TARGET_BASE_URL must be a valid URL");
  }
  return hostname;
}

export function targetBaseUrl() {
  const url = requireEnv("TARGET_BASE_URL").trim().replace(/\/+$/, "");
  const hostname = hostnameFromUrl(url);
  if (hostname === "gongzzang.com" || hostname.endsWith(".gongzzang.com")) {
    throw new Error("production targets are forbidden for load tests");
  }
  return url;
}

export function profile() {
  return __ENV.LOAD_PROFILE || "smoke";
}

export function loadEnvironment() {
  const environment = __ENV.LOAD_ENVIRONMENT || "perf";
  if (!["perf", "staging", "local", "ci"].includes(environment)) {
    throw new Error("LOAD_ENVIRONMENT must be one of: perf, staging, local, ci");
  }
  return environment;
}

export function runTags(scenario) {
  return {
    scenario,
    environment: loadEnvironment(),
    git_sha: __ENV.GIT_SHA || "unknown",
  };
}
