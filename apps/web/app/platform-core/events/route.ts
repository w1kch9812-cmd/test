import { createHmac, timingSafeEqual } from "node:crypto";
import { revalidatePath, revalidateTag } from "next/cache";
import { type NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { env } from "@/lib/env";
import {
  type PlatformCoreEventInboxRecord,
  recordPlatformCoreEventAccepted,
  recordPlatformCoreEventDeadLetter,
  releasePlatformCoreEventReservation,
  reservePlatformCoreEvent,
} from "@/lib/platform-core/event-inbox";
import { ROUTES } from "@/lib/routes";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const GOLD_POINTER_EVENT_TYPE = "catalog.industrial_complex.gold_pointer.published.v1";
const PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE = "catalog.parcel_marker_anchor.snapshot.published.v1";
const PLATFORM_CORE_SCOPE = "catalog";
const CATALOG_CACHE_TAG = "platform-core-catalog";
const PLATFORM_CORE_SIGNATURE_HEADER = "x-platform-core-signature";
const PLATFORM_CORE_TIMESTAMP_HEADER = "x-platform-core-timestamp";
const WEBHOOK_SIGNATURE_VERSION = "v1";
const WEBHOOK_MAX_SKEW_SECONDS = 300;

const PlatformCoreEventEnvelopeSchema = z
  .object({
    event_id: z.string().uuid(),
    event_type: z.string().min(1),
    occurred_at: z.string().datetime({ offset: true }),
    scope: z.literal(PLATFORM_CORE_SCOPE),
    payload: z.object({ type: z.string().min(1) }).passthrough(),
  })
  .passthrough();

const GoldPointerEventSchema = z
  .object({
    event_id: z.string().uuid(),
    event_type: z.literal(GOLD_POINTER_EVENT_TYPE),
    occurred_at: z.string().datetime({ offset: true }),
    scope: z.literal(PLATFORM_CORE_SCOPE),
    payload: z
      .object({
        type: z.literal(GOLD_POINTER_EVENT_TYPE),
        schema_version: z.number().int().min(1),
        complex_id: z.string().min(1),
        current_version: z.string().min(1),
        source_snapshot_id: z.string().min(1),
        iceberg_snapshot_id: z.string().min(1),
      })
      .passthrough(),
  })
  .passthrough();

const ParcelAnchorSnapshotEventSchema = z
  .object({
    event_id: z.string().uuid(),
    event_type: z.literal(PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE),
    occurred_at: z.string().datetime({ offset: true }),
    scope: z.literal(PLATFORM_CORE_SCOPE),
    payload: z
      .object({
        type: z.literal(PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE),
        schema_version: z.number().int().min(1),
        anchor_snapshot_id: z.string().min(1),
        source_geometry_version: z.string().min(1),
        artifact_manifest_url: z
          .string()
          .url()
          .refine((value) => new URL(value).protocol === "https:"),
        artifact_checksum_sha256: z.string().regex(/^[a-f0-9]{64}$/),
        row_count: z.number().int().nonnegative(),
        published_at: z.string().datetime({ offset: true }),
      })
      .passthrough(),
  })
  .passthrough();

type PlatformCoreEventEnvelope = z.infer<typeof PlatformCoreEventEnvelopeSchema>;
type GoldPointerEvent = z.infer<typeof GoldPointerEventSchema>;
type ParcelAnchorSnapshotEvent = z.infer<typeof ParcelAnchorSnapshotEventSchema>;
type AcceptedEffect = "invalidate_catalog_cache" | "enqueue_anchor_projection_import";
type AcceptedResponse = {
  event_id: string;
  effect: AcceptedEffect;
  status: "accepted";
};
type DuplicateResponse = {
  event_id: string;
  effect?: string;
  reason?: string;
  status: "duplicate";
};
type HandlerResult = AcceptedResponse | undefined | "durable_inbox_unavailable";
type EventHandler = (value: unknown) => Promise<HandlerResult>;

const EVENT_HANDLERS: Record<string, EventHandler> = {
  [GOLD_POINTER_EVENT_TYPE]: handleGoldPointerEvent,
  [PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE]: handleParcelAnchorSnapshotEvent,
};

export async function POST(req: NextRequest) {
  const body = await readSignedJson(req);
  if (!body.ok) return rejected(body.reason, body.status);

  const envelope = PlatformCoreEventEnvelopeSchema.safeParse(body.value);
  if (!envelope.success) return rejected("invalid_event");

  const headerCheck = validateRequiredHeaders(req, envelope.data);
  if (!headerCheck.ok) return rejected(headerCheck.reason);

  const reservation = await reservePlatformCoreEvent({
    event_id: envelope.data.event_id,
    event_type: envelope.data.event_type,
    scope: envelope.data.scope,
  });
  if (reservation.status === "duplicate") {
    return NextResponse.json(duplicate(reservation.record), { status: 200 });
  }

  const handler = EVENT_HANDLERS[envelope.data.event_type];
  if (!handler) {
    await deadLetter(envelope.data, "unsupported_event_type");
    return rejected("unsupported_event_type");
  }

  const accepted = await handler(body.value);
  if (accepted === "durable_inbox_unavailable") {
    await releasePlatformCoreEventReservation(envelope.data.event_id);
    return rejected("durable_inbox_unavailable", 503);
  }
  if (!accepted) {
    await deadLetter(envelope.data, "invalid_event");
    return rejected("invalid_event");
  }

  await recordPlatformCoreEventAccepted({
    event_id: envelope.data.event_id,
    event_type: envelope.data.event_type,
    scope: envelope.data.scope,
    effect: accepted.effect,
  });

  return NextResponse.json(accepted, { status: 202 });
}

async function handleGoldPointerEvent(value: unknown): Promise<AcceptedResponse | undefined> {
  const parsed = GoldPointerEventSchema.safeParse(value);
  if (!parsed.success) return undefined;

  invalidateCatalogCache();

  return accepted(parsed.data, "invalidate_catalog_cache");
}

async function handleParcelAnchorSnapshotEvent(
  value: unknown,
): Promise<AcceptedResponse | undefined | "durable_inbox_unavailable"> {
  const parsed = ParcelAnchorSnapshotEventSchema.safeParse(value);
  if (!parsed.success) return undefined;

  const persisted = await persistPlatformCoreEvent(parsed.data);
  if (!persisted) return "durable_inbox_unavailable";

  return accepted(parsed.data, "enqueue_anchor_projection_import");
}

async function persistPlatformCoreEvent(event: PlatformCoreEventEnvelope): Promise<boolean> {
  try {
    const res = await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/platform-core/events`, {
      body: JSON.stringify(event),
      headers: {
        "content-type": "application/json",
        "x-internal-auth": env.INTERNAL_AUTH_SECRET,
      },
      method: "POST",
    });
    return res.ok;
  } catch {
    return false;
  }
}

async function readSignedJson(
  req: NextRequest,
): Promise<{ ok: true; value: unknown } | { ok: false; reason: string; status: number }> {
  const bodyText = await req.text();
  if (!isValidWebhookSignature(req, bodyText)) {
    return { ok: false, reason: "invalid_signature", status: 401 };
  }

  try {
    return { ok: true, value: JSON.parse(bodyText) as unknown };
  } catch {
    return { ok: false, reason: "invalid_json", status: 400 };
  }
}

function isValidWebhookSignature(req: NextRequest, bodyText: string): boolean {
  const timestamp = requiredHeader(req, PLATFORM_CORE_TIMESTAMP_HEADER);
  const signature = requiredHeader(req, PLATFORM_CORE_SIGNATURE_HEADER);
  if (!timestamp || !signature || !isFreshTimestamp(timestamp)) {
    return false;
  }

  const actualHex = signatureValue(signature);
  if (!actualHex) {
    return false;
  }

  const expectedHex = createHmac("sha256", env.PLATFORM_CORE_WEBHOOK_SECRET)
    .update(`${timestamp}.${bodyText}`)
    .digest("hex");
  const actual = Buffer.from(actualHex, "hex");
  const expected = Buffer.from(expectedHex, "hex");
  return actual.length === expected.length && timingSafeEqual(actual, expected);
}

function isFreshTimestamp(value: string): boolean {
  if (!/^\d+$/.test(value)) {
    return false;
  }
  const timestamp = Number(value);
  if (!Number.isSafeInteger(timestamp)) {
    return false;
  }
  const now = Math.floor(Date.now() / 1000);
  return Math.abs(now - timestamp) <= WEBHOOK_MAX_SKEW_SECONDS;
}

function signatureValue(value: string): string | undefined {
  const prefix = `${WEBHOOK_SIGNATURE_VERSION}=`;
  if (!value.startsWith(prefix)) {
    return undefined;
  }
  const hex = value.slice(prefix.length);
  return /^[a-f0-9]{64}$/.test(hex) ? hex : undefined;
}

function validateRequiredHeaders(
  req: NextRequest,
  event: PlatformCoreEventEnvelope,
): { ok: true } | { ok: false; reason: string } {
  const eventId = requiredHeader(req, "x-platform-core-event-id");
  const eventType = requiredHeader(req, "x-platform-core-event-type");
  const scope = requiredHeader(req, "x-platform-core-outbox-scope");

  if (!eventId || !eventType || !scope) {
    return { ok: false, reason: "missing_required_header" };
  }
  if (eventId !== event.event_id || eventType !== event.event_type || scope !== event.scope) {
    return { ok: false, reason: "header_body_mismatch" };
  }
  if (event.payload.type !== event.event_type) {
    return { ok: false, reason: "payload_type_mismatch" };
  }

  return { ok: true };
}

function requiredHeader(req: NextRequest, name: string): string | undefined {
  const value = req.headers.get(name)?.trim();
  return value ? value : undefined;
}

function invalidateCatalogCache() {
  revalidatePath(ROUTES.listings.index, "page");
  revalidateTag(CATALOG_CACHE_TAG, { expire: 0 });
}

function accepted(
  event: Pick<GoldPointerEvent | ParcelAnchorSnapshotEvent, "event_id">,
  effect: AcceptedEffect,
): AcceptedResponse {
  return {
    event_id: event.event_id,
    effect,
    status: "accepted",
  };
}

function duplicate(record: PlatformCoreEventInboxRecord): DuplicateResponse {
  return {
    event_id: record.event_id,
    effect: record.effect,
    reason: record.reason,
    status: "duplicate",
  };
}

async function deadLetter(event: PlatformCoreEventEnvelope, reason: string) {
  await recordPlatformCoreEventDeadLetter({
    event_id: event.event_id,
    event_type: event.event_type,
    scope: event.scope,
    reason,
  });
}

function rejected(reason: string, status = 400) {
  return NextResponse.json({ reason, status: "rejected" }, { status });
}
