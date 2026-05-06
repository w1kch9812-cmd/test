// SP6-i: OpenTelemetry SDK init.
// SP-Obs T5 (Rust backend): Sentry init 적용 -- backend panic/error capture
// production 으로 routing. _sentry_guard 가 main lifetime 동안 유지.
//
// SP-Obs FU (frontend Sentry, 별도 commit): @sentry/nextjs dep 추가 + 본
// register 안에서 dynamic import + Sentry.init({ dsn: NEXT_PUBLIC_SENTRY_DSN,
// release, environment, ... }). 1차 = OTLP + backend Sentry 만. frontend
// Sentry 는 SP8 IaC 가 DSN 주입 + source map upload CI step + release tagging
// 같이.

export async function register() {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    const { NodeSDK } = await import("@opentelemetry/sdk-node");
    const { FetchInstrumentation } = await import("@opentelemetry/instrumentation-fetch");
    const sdk = new NodeSDK({
      serviceName: "gongzzang-web",
      instrumentations: [new FetchInstrumentation()],
    });
    sdk.start();
  }
}
