import { revalidatePath, revalidateTag } from "next/cache";
import { type NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { ROUTES } from "@/lib/routes";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const PLATFORM_CORE_EVENT_TYPE = "catalog.industrial_complex.gold_pointer.published.v1";
const PLATFORM_CORE_SCOPE = "catalog";
const CATALOG_CACHE_TAG = "platform-core-catalog";

const GoldPointerEventSchema = z
  .object({
    event_id: z.string().uuid(),
    event_type: z.literal(PLATFORM_CORE_EVENT_TYPE),
    occurred_at: z.string().datetime({ offset: true }),
    scope: z.literal(PLATFORM_CORE_SCOPE),
    payload: z
      .object({
        type: z.literal(PLATFORM_CORE_EVENT_TYPE),
        schema_version: z.number().int().min(1),
        complex_id: z.string().min(1),
        current_version: z.string().min(1),
        source_snapshot_id: z.string().min(1),
        iceberg_snapshot_id: z.string().min(1),
      })
      .passthrough(),
  })
  .passthrough();

type GoldPointerEvent = z.infer<typeof GoldPointerEventSchema>;

export async function POST(req: NextRequest) {
  const body = await readJson(req);
  if (!body.ok) return rejected("invalid_json");

  const parsed = GoldPointerEventSchema.safeParse(body.value);
  if (!parsed.success) return rejected("invalid_event");

  const headerCheck = validateRequiredHeaders(req, parsed.data);
  if (!headerCheck.ok) return rejected(headerCheck.reason);

  invalidateCatalogCache();

  return NextResponse.json(
    {
      event_id: parsed.data.event_id,
      effect: "invalidate_catalog_cache",
      status: "accepted",
    },
    { status: 202 },
  );
}

async function readJson(req: NextRequest): Promise<{ ok: true; value: unknown } | { ok: false }> {
  try {
    return { ok: true, value: await req.json() };
  } catch {
    return { ok: false };
  }
}

function validateRequiredHeaders(
  req: NextRequest,
  event: GoldPointerEvent,
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

function rejected(reason: string) {
  return NextResponse.json({ reason, status: "rejected" }, { status: 400 });
}
