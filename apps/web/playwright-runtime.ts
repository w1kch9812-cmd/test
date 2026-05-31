export const DEFAULT_E2E_PLAYWRIGHT_PORT = 3100;
export const DEFAULT_PROBE_PLAYWRIGHT_PORT = 3101;

const DEFAULT_PLAYWRIGHT_HOST = "127.0.0.1";
const PORT_ERROR = "PLAYWRIGHT_PORT must be an integer between 1 and 65535";

type PlaywrightRuntimeEnv = Record<string, string | undefined>;

interface ResolvePlaywrightRuntimeOptions {
  env: PlaywrightRuntimeEnv;
  defaultPort: number;
}

export interface PlaywrightRuntime {
  host: string;
  port: number;
  baseURL: string;
  webServerUrl: string;
  command: string;
  reuseExistingServer: boolean;
  zitadelRedirectUri: string;
}

function parsePort(rawPort: string | undefined, defaultPort: number): number {
  const value = rawPort?.trim();
  if (!value) return defaultPort;
  if (!/^\d+$/.test(value)) {
    throw new Error(PORT_ERROR);
  }

  const port = Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error(PORT_ERROR);
  }
  return port;
}

function parseHost(rawHost: string | undefined): string {
  const host = rawHost?.trim() || DEFAULT_PLAYWRIGHT_HOST;
  if (!/^[a-zA-Z0-9.-]+$/.test(host)) {
    throw new Error("PLAYWRIGHT_HOST must be a hostname or IPv4 literal without spaces");
  }
  return host;
}

export function resolvePlaywrightRuntime({
  env,
  defaultPort,
}: ResolvePlaywrightRuntimeOptions): PlaywrightRuntime {
  const host = parseHost(env.PLAYWRIGHT_HOST);
  const port = parsePort(env.PLAYWRIGHT_PORT, defaultPort);
  const baseURL = `http://${host}:${port}`;

  return {
    host,
    port,
    baseURL,
    webServerUrl: baseURL,
    command: `pnpm dev -H ${host} -p ${port}`,
    reuseExistingServer: env.CI ? false : env.PLAYWRIGHT_REUSE_EXISTING_SERVER === "1",
    zitadelRedirectUri: `${baseURL}/api/auth/callback`,
  };
}
