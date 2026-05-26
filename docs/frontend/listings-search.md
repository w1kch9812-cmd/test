# Listings Search Runtime Guide

## Current Runtime Shape

`/listings` is a PNU-first listing screen.

```text
/login -> /listings
          -> proxy.ts auth gate
          -> GET /api/proxy/listings?pnu=&admin_code=&types=&page=
          -> backend GET /listings
          -> ListingRepository::find_card_summaries
          -> SQL filters on listing.parcel_pnu and denormalized parcel columns
```

Map marker placement is not owned by listing cards. The map consumes the platform-core
vector tile manifest for PNU-anchor PBF layers and opens parcel/listing panels by PNU or object ID.

## Listing Marker Serving

Listing map markers use the Gongzzang `listing_marker_projection` read model. The `listing` table
remains the write-model SSOT for listing semantics, while marker serving reads projection rows that
copy platform-core PNU anchor positions with source lineage.

The browser instant filter applies fast filters such as asset type, deal type, price, and area to
already loaded listing marker tiles. Exact nationwide counts, unseen-tile results, and optional
marker masks come from server marker indexes, not from viewport `bbox` requests.

## Environment

```text
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=<NCP Maps Client ID>
NEXT_PUBLIC_PLATFORM_CORE_BASE_URL=<platform-core origin>
```

## Common Checks

| Symptom | Likely Cause | Check |
|---|---|---|
| Map is blank | Naver client ID missing or SDK blocked | Browser console for Naver SDK load errors |
| Marker layer absent | Platform-core vector tile manifest unavailable or missing anchor artifacts | Network request to `/catalog/v1/vector-tiles/manifest` or configured `NEXT_PUBLIC_TILES_MANIFEST_URL` |
| Listing count is zero | No active listings or filters too narrow | `SELECT count(*) FROM listing WHERE status='active'` |
| Filters ignored | URL query and store diverged | Network request query string |

## Data Sources

- Listing cards: Gongzzang API and `listing` table.
- Marker locations: platform-core parcel marker anchors, served as PBF vector tiles.
- Parcel identity: PNU.
- Photos: listing-photo table integration.

## Main Files

| File | Role |
|---|---|
| `apps/web/components/listings/listing-map.tsx` | Naver map runtime and PBF source/layer setup |
| `apps/web/lib/map/vector-tile-manifest.ts` | Platform-core vector tile manifest client |
| `apps/web/lib/map/marker-tile-contract.ts` | Gongzzang listing marker tile source contract |
| `apps/web/lib/map/marker-tile-style.ts` | Platform-core anchor and Gongzzang listing layer registration |
| `apps/web/lib/listings/use-listings-query.ts` | Listing card query hook |
| `apps/web/stores/listings.ts` | Listing filters and selected listing state |

## Debug Commands

```bash
psql "$DATABASE_URL" -c "SELECT count(*) FROM listing WHERE status='active'"

curl -H "Authorization: Bearer <jwt>" \
  "http://localhost:8080/listings?pnu=1111010100100070000&page=0&size=5" | jq

cargo run -p api
pnpm --filter @gongzzang/web dev
```

## SSOT Rule

Listing card data must not carry marker coordinates. Product marker position is resolved through
PNU-anchor PBF tiles, and detailed listing JSON is fetched after selection.
