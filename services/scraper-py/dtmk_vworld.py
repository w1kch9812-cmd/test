"""V-World dtmk SHP zip 자동 다운로드 → R2 Bronze archive (SP9 ADR 0021).

핵심 정신:
- *정적 데이터 (필지 polygon)* 는 *우리 R2 영구 저장*. runtime API 호출 0.
- 매일 cron 가능: dtmk 페이지 fileSize 비교 → 변경된 시군구만 부분 다운.
- subprocess pattern (tippecanoe / ogr2ogr 와 동일) — Rust ETL 이 spawn.

자세한 architecture: docs/adr/0022-bronze-scraping-isolated-python.md (작성 예정)
"""

from __future__ import annotations

import base64
import json
import os
import re
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import NamedTuple

import boto3
from botocore.config import Config
from curl_cffi import requests as cffi


# ===== .env loader =====
def load_env() -> dict[str, str]:
    """`.env` 또는 환경변수에서 값 로드. dev convenience — production 은 ECS env."""
    env_path = Path(__file__).resolve().parent.parent.parent / ".env"
    out: dict[str, str] = {}
    if env_path.is_file():
        for line in env_path.read_text(encoding="utf-8").splitlines():
            if "=" in line and not line.lstrip().startswith("#"):
                k, _, v = line.partition("=")
                out[k.strip()] = v.strip()
    out.update(os.environ)  # OS env > .env
    return out


ENV = load_env()


def required(name: str) -> str:
    v = ENV.get(name)
    if not v:
        sys.stderr.write(f"ERROR: env var {name} 미설정 — .env 또는 OS env\n")
        sys.exit(2)
    return v


# ===== V-World 사이트 client =====
LOGIN_URL = "https://www.vworld.kr/v4po_usrlogin_a004.do"
DTMK_URL = "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId={ds_id}&datPageSize=1000"
DOWNLOAD_URL = "https://www.vworld.kr/dtmk/downloadResourceFile.do?ds_id={ds_id}&fileNo={file_no}"


class FileEntry(NamedTuple):
    ds_id: str
    file_no: str
    file_size: int  # KB? 또는 bytes — V-World 의 단위 검증 필요. onclick 의 세 번째 인자.
    sigungu: str | None = None  # filename 추출 후 채움


class _IterStream:
    """`Iterator[bytes]` → file-like read() — boto3 `upload_fileobj` 와 호환.

    curl_cffi 의 `iter_content` 결과를 boto3 multipart upload 에 흘림. 메모리는
    chunk 단위만 사용 (~64KB).
    """

    def __init__(self, it):
        self._it = it
        self._buffer = b""

    def read(self, n: int = -1) -> bytes:
        if n is None or n < 0:
            chunks = [self._buffer]
            self._buffer = b""
            for chunk in self._it:
                chunks.append(chunk)
            return b"".join(chunks)
        while len(self._buffer) < n:
            try:
                self._buffer += next(self._it)
            except StopIteration:
                break
        out = self._buffer[:n]
        self._buffer = self._buffer[n:]
        return out


def make_session() -> cffi.Session:
    """curl_cffi 의 Chrome120 impersonate session — V-World 가 fingerprint 검사 시 안전."""
    return cffi.Session(impersonate="chrome120")


def login(session: cffi.Session, username: str, password: str) -> None:
    """V-World 로그인 — base64 encode username/password POST.

    검증된 endpoint: /v4po_usrlogin_a004.do (common_login.js 분석 결과).
    """
    # 메인 페이지 GET — 초기 cookie (PJSESSIONID / SSCSID / WMONID) 발급.
    session.get("https://www.vworld.kr/v4po_main.do", timeout=30)
    data = {
        "usrIdeE": base64.b64encode(username.encode("utf-8")).decode("ascii"),
        "usrPwdE": base64.b64encode(password.encode("utf-8")).decode("ascii"),
        "nextUrl": "",
    }
    headers = {
        "Referer": "https://www.vworld.kr/v4po_main.do",
        "X-Requested-With": "XMLHttpRequest",
        "Origin": "https://www.vworld.kr",
    }
    r = session.post(LOGIN_URL, data=data, headers=headers, timeout=30)
    r.raise_for_status()
    body = r.json()
    rm = body.get("resultMap", {})
    if rm.get("result") != "success":
        sys.stderr.write(f"login failed: {rm.get('msg', body)}\n")
        sys.exit(2)
    print(f"[login] {rm.get('usrNam', '?')} 로그인 OK", flush=True)


def fetch_file_list(session: cffi.Session, ds_id: str) -> list[FileEntry]:
    """dtmk 페이지 GET → onclick 의 listFnc.download(dsId, fileNo, fileSize) 추출."""
    url = DTMK_URL.format(ds_id=ds_id)
    r = session.get(url, timeout=30)
    r.raise_for_status()
    html = r.text
    pattern = re.compile(
        r"listFnc\.download\s*\(\s*'(\d+)'\s*,\s*'(\d+)'\s*,\s*'(\d+)'\s*\)"
    )
    entries: list[FileEntry] = []
    for m in pattern.finditer(html):
        ds, file_no, size_str = m.group(1), m.group(2), m.group(3)
        if ds != ds_id:
            continue
        entries.append(FileEntry(ds_id=ds, file_no=file_no, file_size=int(size_str)))
    return entries


def filename_from_disposition(header: str | None) -> str | None:
    """Content-Disposition: attachment; filename=LSMD_CONT_LDREG_충북_충주시.zip; → 그 이름.

    URL-encoded 파일명 (예: `%EC%B6%A9%EB%B6%81`) 자동 decode.
    """
    if not header:
        return None
    m = re.search(r"filename=([^;]+)", header)
    if not m:
        return None
    raw = m.group(1).strip().strip('"')
    try:
        from urllib.parse import unquote

        return unquote(raw)
    except Exception:
        return raw


def sigungu_from_filename(name: str) -> str:
    """`LSMD_CONT_LDREG_충북_충주시.zip` → `충북_충주시`."""
    m = re.match(r"LSMD_CONT_LDREG_(.+?)\.zip", name)
    return m.group(1) if m else name.removesuffix(".zip")


# ===== R2 client =====
def make_r2():
    cfg = Config(
        signature_version="s3v4",
        retries={"max_attempts": 5, "mode": "standard"},
        s3={"addressing_style": "path"},
    )
    return boto3.client(
        "s3",
        endpoint_url=f"https://{required('R2_ACCOUNT_ID')}.r2.cloudflarestorage.com",
        aws_access_key_id=required("R2_ACCESS_KEY"),
        aws_secret_access_key=required("R2_SECRET_KEY"),
        region_name="auto",
        config=cfg,
    )


def r2_head(r2, bucket: str, key: str) -> dict | None:
    """R2 의 object 메타 조회. 없으면 None."""
    try:
        return r2.head_object(Bucket=bucket, Key=key)
    except Exception:
        return None


# ===== main =====
def main() -> int:
    ds_id = "30563"  # 연속지적도_전국
    bucket = required("R2_BUCKET")
    bronze_prefix = ENV.get("R2_BRONZE_PREFIX", "bronze").rstrip("/")
    # batch label = YYYY-MM (월 1 archive). 매일 incremental 시는 같은 batch 안에서 file 변경.
    batch = time.strftime("%Y-%m")
    parallel = int(ENV.get("DTMK_PARALLEL", "3"))  # V-World 서버 부담 고려 default 3.

    username = required("VWORLD_USERNAME")
    password = required("VWORLD_PASSWORD")

    session = make_session()
    login(session, username, password)

    print(f"[probe] dtmk dsId={ds_id} 의 file list 가져오는 중...", flush=True)
    entries = fetch_file_list(session, ds_id)
    print(f"[probe] 총 {len(entries)} files (size KB sum = {sum(e.file_size for e in entries):,})", flush=True)
    if not entries:
        sys.stderr.write("ERROR: file list 비어있음 (페이지 변경?)\n")
        return 2

    r2 = make_r2()

    def upload_one(idx: int, entry: FileEntry) -> tuple[str, bool, int]:
        """단일 파일 다운 + R2 PUT. (sigungu, downloaded?, bytes)"""
        # 우선 헤더만 fetch — Content-Disposition 의 filename 으로 시군구명 추출.
        url = DOWNLOAD_URL.format(ds_id=entry.ds_id, file_no=entry.file_no)
        # idempotent skip — R2 의 같은 키 + 비슷한 size 면 다운 안 함.
        # 실 size 는 Content-Length 헤더로 비교. 단 여기선 onclick 의 size_kb 를 사용.
        # 정확한 비교는 다음 단계 (manifest 의 sha256 비교).
        # *file_no 만으로 R2 key 결정* — sigungu 명은 응답 헤더에서 받음.
        # curl_cffi 의 Response 는 `with` context manager 미지원 — 직접 변수 할당.
        r = session.get(url, stream=True, timeout=600)
        r.raise_for_status()
        disp = r.headers.get("Content-Disposition")
        filename = filename_from_disposition(disp) or f"file-{entry.file_no}.zip"
        sigungu = sigungu_from_filename(filename)
        content_len = int(r.headers.get("Content-Length", "0"))
        key = f"{bronze_prefix}/{batch}/parcel-dtmk-{ds_id}/{filename}"

        # idempotent — 이미 같은 size 의 object 가 R2 에 있으면 skip.
        existing = r2_head(r2, bucket, key)
        if existing and int(existing.get("ContentLength", 0)) == content_len and content_len > 0:
            r.close()
            print(
                f"[{idx:3d}/{len(entries)}] skip {sigungu} ({content_len:,}B, R2 동일)",
                flush=True,
            )
            return sigungu, False, 0

        # streaming PUT — iter_content 로 chunk 받아 BytesIO buffer 거쳐 upload_fileobj.
        # boto3 가 multipart 자동 — 메모리는 chunk 단위만 사용.
        # S3 metadata 는 ASCII only — sigungu 한글은 URL-encode 후 박제.
        from urllib.parse import quote as url_quote
        try:
            r2.upload_fileobj(
                _IterStream(r.iter_content(chunk_size=64 * 1024)),
                bucket,
                key,
                ExtraArgs={
                    "ContentType": "application/zip",
                    "Metadata": {
                        "ds_id": entry.ds_id,
                        "file_no": entry.file_no,
                        "sigungu_urlencoded": url_quote(sigungu, safe=""),
                        "fetched_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                    },
                },
            )
        finally:
            r.close()
        print(
            f"[{idx:3d}/{len(entries)}] PUT {sigungu} ({content_len:,}B -> s3://{bucket}/{key})",
            flush=True,
        )
        return sigungu, True, content_len

    # concurrent download (V-World 서버 부담 고려 default 3 parallel).
    started = time.time()
    results: list[tuple[str, bool, int]] = []
    with ThreadPoolExecutor(max_workers=parallel) as ex:
        futs = {ex.submit(upload_one, i + 1, e): e for i, e in enumerate(entries)}
        for fut in as_completed(futs):
            try:
                results.append(fut.result())
            except Exception as e:
                entry = futs[fut]
                sys.stderr.write(f"FAIL fileNo={entry.file_no}: {e}\n")
                results.append((f"fileNo-{entry.file_no}", False, -1))

    elapsed = time.time() - started
    downloaded = sum(1 for r in results if r[1] and r[2] > 0)
    skipped = sum(1 for r in results if not r[1])
    failed = sum(1 for r in results if r[2] < 0)
    total_bytes = sum(max(0, r[2]) for r in results)

    summary = {
        "ds_id": ds_id,
        "batch": batch,
        "total_files": len(entries),
        "downloaded": downloaded,
        "skipped_idempotent": skipped,
        "failed": failed,
        "total_bytes": total_bytes,
        "elapsed_seconds": int(elapsed),
        "r2_bucket": bucket,
        "r2_prefix": f"{bronze_prefix}/{batch}/parcel-dtmk-{ds_id}/",
    }
    print("\n" + json.dumps(summary, indent=2, ensure_ascii=False), flush=True)
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
