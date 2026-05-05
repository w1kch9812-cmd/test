import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async () => {
  const locale = "ko";
  const [common, auth] = await Promise.all([
    import("./lib/i18n/ko.json"),
    import("./lib/i18n/messages/auth.ko.json"),
  ]);
  return {
    locale,
    messages: { ...common.default, ...auth.default },
  };
});
