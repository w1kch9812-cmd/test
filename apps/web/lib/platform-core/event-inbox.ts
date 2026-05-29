import { getRedis } from "@/lib/session/redis";

const INBOX_KEY_PREFIX = "platform-core:event-inbox:";
const INBOX_TTL_SECONDS = 60 * 60 * 24 * 30;

export type PlatformCoreEventInboxStatus = "processing" | "accepted" | "dead_letter";

export interface PlatformCoreEventInboxRecord {
  event_id: string;
  event_type: string;
  scope: string;
  status: PlatformCoreEventInboxStatus;
  first_seen_at: string;
  updated_at: string;
  effect?: string;
  reason?: string;
}

export type PlatformCoreEventInboxReservation =
  | { status: "started"; record: PlatformCoreEventInboxRecord }
  | { status: "duplicate"; record: PlatformCoreEventInboxRecord };

export async function reservePlatformCoreEvent(input: {
  event_id: string;
  event_type: string;
  scope: string;
}): Promise<PlatformCoreEventInboxReservation> {
  const now = new Date().toISOString();
  const record: PlatformCoreEventInboxRecord = {
    event_id: input.event_id,
    event_type: input.event_type,
    scope: input.scope,
    status: "processing",
    first_seen_at: now,
    updated_at: now,
  };
  const key = inboxKey(input.event_id);
  const created = await getRedis().set(key, JSON.stringify(record), "EX", INBOX_TTL_SECONDS, "NX");
  if (created === "OK") {
    return { status: "started", record };
  }

  const existing = await getPlatformCoreEventInboxRecord(input.event_id);
  if (existing) {
    return { status: "duplicate", record: existing };
  }

  return { status: "started", record };
}

export async function recordPlatformCoreEventAccepted(input: {
  event_id: string;
  event_type: string;
  scope: string;
  effect: string;
}): Promise<PlatformCoreEventInboxRecord> {
  return writePlatformCoreEventRecord({
    event_id: input.event_id,
    event_type: input.event_type,
    scope: input.scope,
    status: "accepted",
    effect: input.effect,
  });
}

export async function recordPlatformCoreEventDeadLetter(input: {
  event_id: string;
  event_type: string;
  scope: string;
  reason: string;
}): Promise<PlatformCoreEventInboxRecord> {
  return writePlatformCoreEventRecord({
    event_id: input.event_id,
    event_type: input.event_type,
    scope: input.scope,
    status: "dead_letter",
    reason: input.reason,
  });
}

export async function releasePlatformCoreEventReservation(eventId: string): Promise<void> {
  const existing = await getPlatformCoreEventInboxRecord(eventId);
  if (existing?.status === "processing") {
    await getRedis().del(inboxKey(eventId));
  }
}

export async function getPlatformCoreEventInboxRecord(
  eventId: string,
): Promise<PlatformCoreEventInboxRecord | undefined> {
  const raw = await getRedis().get(inboxKey(eventId));
  if (!raw) {
    return undefined;
  }
  return JSON.parse(raw) as PlatformCoreEventInboxRecord;
}

async function writePlatformCoreEventRecord(
  input: Omit<PlatformCoreEventInboxRecord, "first_seen_at" | "updated_at">,
): Promise<PlatformCoreEventInboxRecord> {
  const existing = await getPlatformCoreEventInboxRecord(input.event_id);
  const now = new Date().toISOString();
  const record: PlatformCoreEventInboxRecord = {
    ...input,
    first_seen_at: existing?.first_seen_at ?? now,
    updated_at: now,
  };
  await getRedis().set(inboxKey(input.event_id), JSON.stringify(record), "EX", INBOX_TTL_SECONDS);
  return record;
}

function inboxKey(eventId: string): string {
  return `${INBOX_KEY_PREFIX}${eventId}`;
}
