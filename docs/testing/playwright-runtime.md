# Playwright Runtime SSOT

## Purpose

Playwright must run against the Gongzzang web app that the test run owns. It must not silently attach to another local project that happens to listen on `localhost:3000`.

## Single Source

`apps/web/playwright-runtime.ts` is the SSOT for Playwright endpoint selection.

Defaults:

| Target | Host | Port | URL |
|---|---:|---:|---|
| E2E | `127.0.0.1` | `3100` | `http://127.0.0.1:3100` |
| Probe | `127.0.0.1` | `3101` | `http://127.0.0.1:3101` |

Both `apps/web/playwright.config.ts` and `apps/web/playwright.probes.config.ts` derive:

- `use.baseURL`
- `webServer.url`
- `webServer.command`
- local `ZITADEL_REDIRECT_URI`

from that SSOT.

## Server Reuse

Implicit server reuse is disabled by default. This prevents false-positive or hanging E2E runs when another project owns the same port.

Local reuse is allowed only when explicitly requested:

```powershell
$env:PLAYWRIGHT_REUSE_EXISTING_SERVER='1'
pnpm --filter @gongzzang/web test:e2e
```

CI always disables reuse, even if the env var is set.

## Overrides

Use these only for intentional local debugging:

| Env | Example | Effect |
|---|---|---|
| `PLAYWRIGHT_HOST` | `localhost` | Changes dev server bind host |
| `PLAYWRIGHT_PORT` | `4100` | Changes managed test port |
| `PLAYWRIGHT_REUSE_EXISTING_SERVER` | `1` | Reuses an existing matching server outside CI |

Invalid ports and unsafe host values fail before Playwright can attach to the wrong target.

## Verification

Run:

```powershell
pnpm --filter @gongzzang/web exec vitest run tests/unit/playwright-runtime.test.ts tests/unit/playwright-config.test.ts
$env:CI='1'; pnpm --filter @gongzzang/web exec playwright test
```

Expected:

- runtime/config tests pass
- E2E starts a managed Next dev server on `127.0.0.1:3100`
- no dependency on `localhost:3000`
