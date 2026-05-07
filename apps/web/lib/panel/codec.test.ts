// apps/web/lib/panel/codec.test.ts
import { describe, expect, it } from "vitest";
import { g1Codec, ParseError } from "./codec";
import type { PanelStack } from "./types";

describe("g1Codec", () => {
  it("serializes single parcel.summary entry", () => {
    const stack: PanelStack = {
      v: 1,
      entries: [{ kind: "parcel", id: "1168010100107370000", view: "summary" }],
    };
    expect(g1Codec.serialize(stack)).toBe("parcel:1168010100107370000.summary");
  });

  it("serializes 2-entry chain with > separator", () => {
    const stack: PanelStack = {
      v: 1,
      entries: [
        { kind: "parcel", id: "1168010100107370000", view: "summary" },
        { kind: "listing", id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", view: "summary" },
      ],
    };
    expect(g1Codec.serialize(stack)).toBe(
      "parcel:1168010100107370000.summary>listing:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.summary",
    );
  });

  it("serializes empty stack to empty string", () => {
    expect(g1Codec.serialize({ v: 1, entries: [] })).toBe("");
  });

  it("round-trips a 2-entry stack", () => {
    const s =
      "parcel:1168010100107370000.summary>listing:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.summary";
    const parsed = g1Codec.deserialize(s);
    expect(parsed.ok).toBe(true);
    if (parsed.ok) expect(g1Codec.serialize(parsed.value)).toBe(s);
  });

  it("rejects unknown kind", () => {
    const r = g1Codec.deserialize("alien:abc.summary");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.UnknownKind);
  });

  it("rejects unknown view for parcel", () => {
    const r = g1Codec.deserialize("parcel:1168010100107370000.alienView");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.UnknownView);
  });

  it("rejects PNU pattern violation", () => {
    const r = g1Codec.deserialize("parcel:notapnu.summary");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.IdPatternViolation);
  });

  it("rejects malformed entry (missing dot)", () => {
    const r = g1Codec.deserialize("parcel:1168010100107370000");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.Malformed);
  });

  it("rejects depth > PANEL_DEPTH_MAX", () => {
    const long = Array.from({ length: 9 }, () => "parcel:1168010100107370000.summary").join(">");
    const r = g1Codec.deserialize(long);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe(ParseError.DepthExceeded);
  });

  it("returns Malformed for empty input round-trip", () => {
    // empty string is a valid empty stack — caller decides which
    const r = g1Codec.deserialize("");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value.entries).toHaveLength(0);
  });
});
