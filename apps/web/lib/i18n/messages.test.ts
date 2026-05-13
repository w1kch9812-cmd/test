import { describe, expect, it } from "vitest";
import koMessages from "./ko.json";

function collectDottedMessageKeys(value: unknown, path: string[] = []): string[] {
  if (!value || typeof value !== "object" || Array.isArray(value)) return [];

  return Object.entries(value).flatMap(([key, nested]) => {
    const nextPath = [...path, key];
    const dottedKey = key.includes(".") ? [nextPath.join(".")] : [];
    return [...dottedKey, ...collectDottedMessageKeys(nested, nextPath)];
  });
}

describe("next-intl message contract", () => {
  it("does not use dot characters inside message keys", () => {
    expect(collectDottedMessageKeys(koMessages)).toEqual([]);
  });
});
