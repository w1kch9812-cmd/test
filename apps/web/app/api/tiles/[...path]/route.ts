/**
 * SP9 T3b.2 — `pmtiles://` scheme 우회 path.
 *
 * Naver Maps gl SDK 의 mapbox-gl namespace 가 외부에 노출 안 됨 → `addProtocol`
 * 등록 불가. 대신 `mb.addSource(type:'vector', tiles:['/api/tiles/...'])` 표준
 * 패턴 사용. 본 route 가 `<name>.pmtiles` 단일 파일에서 (z,x,y) tile 을 추출
 * 후 `application/x-protobuf` 로 반환.
 *
 * design-lab 의 검증된 구현 (`gongzzang/apps/gongzzang-design-lab/app/api/tiles/[...path]/route.ts`)
 * 을 본 프로젝트에 맞춰 포팅. path 차이만:
 * - 파일 위치: `public/pmtiles/<name>.pmtiles` (design-lab 은 `public/data/geometry/`)
 *
 * production R2 직결 시 본 route 비활성 — `NEXT_PUBLIC_PMTILES_BASE_URL` 이 R2
 * URL 가리키면 클라가 직접 fetch.
 */

import { type NextRequest, NextResponse } from "next/server";

// eslint-disable-next-line @typescript-eslint/no-require-imports -- 동적 require: server-side fs/path 만, R2 모드에서는 미사용
const path = require("node:path") as typeof import("path");
// eslint-disable-next-line @typescript-eslint/no-require-imports
const fs = require("node:fs") as typeof import("fs");

// pmtiles 4.x — Source interface 와 PMTiles class
import { PMTiles, type RangeResponse, type Source } from "pmtiles";

const ALLOWED_TILES = new Set([
  "parcels",
  "admin",
  "complex",
  // T3b.2 transitional — 형제 repo 의 lots.pmtiles 를 borrow.
  "lots",
  "sido",
  "sig",
  "emd",
]);

/** 로컬 파일에서 byte-range 만 읽는 Source. RAM 절약 (전체 file mmap 안 함). */
class FileSource implements Source {
  constructor(private readonly filePath: string) {}

  // pmtiles Source interface
  async getBytes(offset: number, length: number): Promise<RangeResponse> {
    const fd = fs.openSync(this.filePath, "r");
    try {
      const buffer = Buffer.alloc(length);
      fs.readSync(fd, buffer, 0, length, offset);
      return {
        data: buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength),
      };
    } finally {
      fs.closeSync(fd);
    }
  }

  getKey(): string {
    return this.filePath;
  }
}

interface CacheEntry {
  pmtiles: PMTiles;
  lastAccess: number;
}

const cache = new Map<string, CacheEntry>();
const MAX_ENTRIES = 8;
const TTL_MS = 30 * 60 * 1000;

function evictLru(): void {
  const now = Date.now();
  for (const [k, v] of cache) {
    if (now - v.lastAccess > TTL_MS) cache.delete(k);
  }
  if (cache.size > MAX_ENTRIES) {
    const sorted = [...cache.entries()].sort((a, b) => a[1].lastAccess - b[1].lastAccess);
    for (const [k] of sorted.slice(0, sorted.length - MAX_ENTRIES)) cache.delete(k);
  }
}

function pmtilesPath(name: string): string {
  return path.join(process.cwd(), "public", "pmtiles", `${name}.pmtiles`);
}

async function getPmtiles(name: string): Promise<PMTiles | null> {
  evictLru();

  const cached = cache.get(name);
  if (cached) {
    cached.lastAccess = Date.now();
    return cached.pmtiles;
  }

  const filePath = pmtilesPath(name);
  if (!fs.existsSync(filePath)) {
    return null;
  }
  try {
    const source = new FileSource(filePath);
    const p = new PMTiles(source);
    await p.getHeader(); // metadata pre-fetch
    cache.set(name, { pmtiles: p, lastAccess: Date.now() });
    return p;
  } catch (e) {
    console.error(`[pmtiles] load failed: ${name}`, e);
    return null;
  }
}

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ path: string[] }> },
): Promise<NextResponse> {
  const { path: parts } = await params;
  // pattern: /api/tiles/<name>/<z>/<x>/<y>.pbf
  if (parts.length !== 4) {
    return NextResponse.json({ error: "invalid path" }, { status: 400 });
  }
  const name = parts[0] ?? "";
  const zStr = parts[1] ?? "";
  const xStr = parts[2] ?? "";
  const yFile = parts[3] ?? "";

  // 화이트리스트 + path traversal 방어
  if (!ALLOWED_TILES.has(name) || /[./\\]/.test(name)) {
    return NextResponse.json({ error: "invalid tile name" }, { status: 400 });
  }

  const z = Number.parseInt(zStr, 10);
  const x = Number.parseInt(xStr, 10);
  const y = Number.parseInt(yFile.replace(".pbf", ""), 10);
  if (Number.isNaN(z) || Number.isNaN(x) || Number.isNaN(y) || z < 0 || z > 22 || x < 0 || y < 0) {
    return NextResponse.json({ error: "invalid tile coordinates" }, { status: 400 });
  }

  const p = await getPmtiles(name);
  if (!p) {
    return NextResponse.json({ error: "pmtiles not found" }, { status: 404 });
  }

  try {
    const tile = await p.getZxy(z, x, y);
    if (!tile || !tile.data || tile.data.byteLength === 0) {
      // 빈 tile = 해당 영역에 데이터 없음. 204 (No Content).
      return new NextResponse(null, { status: 204 });
    }
    return new NextResponse(new Uint8Array(tile.data), {
      status: 200,
      headers: {
        "Content-Type": "application/x-protobuf",
        // PMTiles.getZxy() 은 압축 해제된 raw MVT bytes 반환 — Content-Encoding 불필요.
        "Cache-Control": "public, max-age=86400, immutable",
      },
    });
  } catch (e) {
    console.error(`[pmtiles] tile read failed (${name}/${z}/${x}/${y}):`, e);
    return new NextResponse("tile read error", { status: 500 });
  }
}
