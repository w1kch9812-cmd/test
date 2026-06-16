export const DEFAULT_E2E_PLAYWRIGHT_PORT = 3100;
export const DEFAULT_PROBE_PLAYWRIGHT_PORT = 3101;

const DEFAULT_PLAYWRIGHT_HOST = "127.0.0.1";
const PORT_ERROR = "PLAYWRIGHT_PORT must be an integer between 1 and 65535";
const BAZEL_NEXT_CLI_PATH = "bazel/next-cli.mjs";
const BAZEL_NODE_EXECUTABLE = "node";

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
  outputDir?: string;
  reportDir?: string;
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

function shellQuote(value: string): string {
  if (/^[a-zA-Z0-9_/:=+,.@%-]+$/.test(value)) {
    return value;
  }
  return `'${value.replace(/'/g, "'\"'\"'")}'`;
}

function joinEnvPath(basePath: string | undefined, leaf: string): string | undefined {
  const base = basePath?.trim();
  if (!base) return undefined;
  return `${base.replace(/[\\/]+$/, "")}/${leaf}`;
}

function resolveCommand(env: PlaywrightRuntimeEnv, host: string, port: number): string {
  if (env.GONGZZANG_BAZEL_PLAYWRIGHT === "1") {
    const nodeExecutable = env.PLAYWRIGHT_NODE_EXECUTABLE?.trim() || BAZEL_NODE_EXECUTABLE;
    const nextCliPath = env.PLAYWRIGHT_NEXT_CLI_PATH?.trim() || BAZEL_NEXT_CLI_PATH;
    return `${shellQuote(nodeExecutable)} ${shellQuote(nextCliPath)} dev -H ${shellQuote(
      host,
    )} -p ${port}`;
  }

  return `pnpm dev -H ${host} -p ${port}`;
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
    command: resolveCommand(env, host, port),
    reuseExistingServer: env.CI ? false : env.PLAYWRIGHT_REUSE_EXISTING_SERVER === "1",
    zitadelRedirectUri: `${baseURL}/api/auth/callback`,
    outputDir: joinEnvPath(env.TEST_UNDECLARED_OUTPUTS_DIR, "test-results"),
    reportDir: joinEnvPath(env.TEST_UNDECLARED_OUTPUTS_DIR, "playwright-report"),
  };
}
