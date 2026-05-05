// SP6-i: OpenTelemetry SDK init.
// SP7-i 가 추가: Sentry connector + OTLP exporter.

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
