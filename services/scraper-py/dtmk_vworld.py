"""V-World dtmk SHP zip 자동 다운로드 → R2 Bronze archive (SP9 ADR 0021 + ADR 0025).

핵심 정신:
- *정적 데이터 (필지 polygon)* 는 *우리 R2 영구 저장*. runtime API 호출 0.
- 매월 cron (workflow phase 1, ADR 0025) — Rust ETL 측이 R2 재소비.

Round 3 P0 hardening:
- DTMK_DS_ID 는 env-driven SSOT (Rust `sp9_base_layer_config::DTMK_DS_ID` reflection)
- V-World HTTP 호출은 tenacity retry — `circuit-breaker` Rust crate 와 동등 정책
- `except Exception` 제거 — typed exception (botocore ClientError / cffi RequestException)
- raw HTML response 보존 — 페이지 변경 시 audit trail (`bronze/.../audit/<date>-list.html`)

자세한 architecture: docs/adr/0022-bronze-scraping-isolated-python-service.md
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
from typing import Any, NamedTuple
from urllib.parse import quote as url_quote
from urllib.parse import unquote

import boto3
from botocore.config import Config
from botocore.exceptions import (
    ClientError,
    ConnectionClosedError,
    ConnectTimeoutError,
    EndpointConnectionError,
    ReadTimeoutError,
    SSLError,
)
from curl_cffi import requests as cffi
from curl_cffi.requests import exceptions as cffi_exc
from tenacity import (
    RetryError,
    Retrying,
    retry_if_exception,
    stop_after_attempt,
    wait_exponential,
)


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

    def __init__(self, it: Any) -> None:
        # `it` 은 `Iterator[bytes]` 인데 curl_cffi 의 iter_content 가 type stubs 가 없어
        # Any. 본 클래스가 byte iterator 를 file-like 로 adapt 하는 single-purpose 라
        # `Any` 가 실용적.
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


# Module-level — `r2_put_with_retry` / `upload_one` / V-World 호출 모두 공유.
# Transport-level transient = TCP / TLS / DNS 단계 실패. boto3 standard retry 가 일부
# 커버하지만 우리 측 wrap 으로 추가 안전망 (특히 audit 같은 invariant path).
transport_exception_types_main: tuple[type[Exception], ...] = (
    EndpointConnectionError,
    ConnectionClosedError,
    ReadTimeoutError,
    ConnectTimeoutError,
    SSLError,
)

# R2 의 transient HTTP error code 화이트리스트. 4xx (AccessDenied / NoSuchBucket /
# 잘못된 key) 는 retry 무의미 → immediate raise.
R2_TRANSIENT_HTTP_CODES = frozenset(
    {"InternalError", "ServiceUnavailable", "SlowDown", "Throttling"}
)
# 5xx 범위 (server fault) 는 무조건 transient.
HTTP_5XX_MIN = 500
HTTP_5XX_MAX = 600


def is_transient_for_retry(exc: BaseException) -> bool:  # noqa: PLR0911
    """Round 4 #3 + #4 — V-World HTTP / R2 PUT / R2 GET 모두 공유하는 retry predicate.

    분류:
    1. **Transport-level** (TCP / TLS / DNS): 무조건 retry.
       - `transport_exception_types_main` (botocore BotoCoreError 서브클래스 + cffi
         의 ConnectionError / Timeout 같은 connection 단계 예외)
       - stdlib `ConnectionError` / `TimeoutError` (Python 표준)
    2. **HTTP-level**:
       - `cffi.HTTPError` (V-World raise_for_status): 4xx 즉시 raise (인증 만료 / 잘못된
         endpoint), 5xx 만 retry. response.status_code 로 분류.
       - `botocore.ClientError` (R2): 5xx / `R2_TRANSIENT_HTTP_CODES` 만 retry.
    3. 그 외 모든 예외 = retry 안 함 (programming error / unknown).

    이전 (Round 3) `retry_if_exception_type((cffi.RequestsError, ...))` 는 4xx HTTP
    error 도 retry 대상에 포함시키는 trick — Round 4 #3 fix.
    """
    # (1a) cffi 의 connection 단계 transient (RequestsError 의 자손인 ConnectionError /
    #      Timeout / DNSError / SSLError) — handshake / read 단계 fail.
    cffi_transport = (
        cffi_exc.ConnectionError,
        cffi_exc.Timeout,
        cffi_exc.DNSError,
        cffi_exc.SSLError,
        cffi_exc.ChunkedEncodingError,
        cffi_exc.ContentDecodingError,
        cffi_exc.IncompleteRead,
    )
    if isinstance(exc, cffi_transport):
        return True
    # (1b) botocore transport-level — 무조건 retry.
    if isinstance(exc, transport_exception_types_main):
        return True
    # (1c) stdlib transport-level.
    if isinstance(exc, (ConnectionError, TimeoutError)):
        return True
    # (2a) cffi.HTTPError — raise_for_status 가 raise, response.status_code 로 분류.
    if isinstance(exc, cffi_exc.HTTPError):
        response = getattr(exc, "response", None)
        if response is None:
            # response 박제 안 됐으면 connection-level fail 로 간주 → retry.
            return True
        status = getattr(response, "status_code", None)
        if status is None:
            return True
        return HTTP_5XX_MIN <= int(status) < HTTP_5XX_MAX
    # (2b) ClientError — 5xx / Throttling 만 retry.
    if isinstance(exc, ClientError):
        code = exc.response.get("Error", {}).get("Code", "")
        status = exc.response.get("ResponseMetadata", {}).get("HTTPStatusCode", 0)
        if code in R2_TRANSIENT_HTTP_CODES:
            return True
        return isinstance(status, int) and HTTP_5XX_MIN <= status < HTTP_5XX_MAX
    return False


# ===== Retry policy (circuit-breaker Rust crate 와 동등) =====
# Round 4 #3 — `_make_retrying` 가 typed `is_transient_for_retry` 사용.
# 이전: `retry_if_exception_type((cffi.RequestsError, ...))` 가 4xx 도 retry.
# 새: 4xx 는 첫 시도에서 즉시 raise → V-World 인증 실패 / dataset 부재 같은 SLO 위반
# 시 빠른 fail-fast.
def _make_retrying() -> Retrying:
    return Retrying(
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=8),
        retry=retry_if_exception(is_transient_for_retry),
        reraise=True,
    )


def http_get_with_retry(session: cffi.Session, url: str, *, timeout: int) -> cffi.Response:
    """tenacity retry 로 wrap 된 GET. 3 시도 + exponential backoff."""
    for attempt in _make_retrying():
        with attempt:
            r = session.get(url, timeout=timeout)
            r.raise_for_status()
            return r
    msg = "_make_retrying must reraise on exhaustion"
    raise RuntimeError(msg)  # pragma: no cover — tenacity reraise 가 보장


def http_post_with_retry(
    session: cffi.Session,
    url: str,
    *,
    data: dict[str, str],
    headers: dict[str, str],
    timeout: int,
) -> cffi.Response:
    """tenacity retry POST."""
    for attempt in _make_retrying():
        with attempt:
            r = session.post(url, data=data, headers=headers, timeout=timeout)
            r.raise_for_status()
            return r
    msg = "_make_retrying must reraise on exhaustion"
    raise RuntimeError(msg)  # pragma: no cover


def r2_put_with_retry(
    r2: Any,
    *,
    bucket: str,
    key: str,
    body: bytes,
    content_type: str,
    cache_control: str | None = None,
    metadata: dict[str, str] | None = None,
) -> None:
    """tenacity retry 로 wrap 된 small-payload R2 PUT.

    boto3 의 standard retry config (5x) 위에 추가 wrapper — *전송 자체* 가 transient
    fail 한 경우 (network reset / 5xx) 우리 측에서 한 번 더 보호. boto3 retry 가
    이미 통과 못 한 케이스 = 5x 다 실패 = 진짜 systemic. retry 1 회 더 = 의미 적지만
    *audit 박제 같은 "must succeed" path 의 추가 안전망*.

    Note: streaming upload (`upload_fileobj`) 는 별도 wrap (chunk position rewind 필요).
    """
    # Round 4 #4 — module-level `is_transient_for_retry` 와 공유. V-World HTTP /
    # R2 PUT / R2 GET 모두 동일 분류 규칙 통과 (drift 차단).

    args: dict[str, Any] = {
        "Bucket": bucket,
        "Key": key,
        "Body": body,
        "ContentType": content_type,
    }
    if cache_control:
        args["CacheControl"] = cache_control
    if metadata:
        args["Metadata"] = metadata

    # tenacity 의 retry_if_exception(은 *transient 만* retry 대상으로 한정.
    # 4xx (AccessDenied / NoSuchBucket) 는 첫 시도 후 즉시 reraise — retry 무의미.
    for attempt in Retrying(
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=8),
        retry=retry_if_exception(is_transient_for_retry),
        reraise=True,
    ):
        with attempt:
            r2.put_object(**args)


def login(session: cffi.Session, username: str, password: str) -> None:
    """V-World 로그인 — base64 encode username/password POST.

    검증된 endpoint: /v4po_usrlogin_a004.do (common_login.js 분석 결과).
    Round 3 P0 — tenacity retry wrap (transient 5xx / connection drop 자동 복구).
    """
    # 메인 페이지 GET — 초기 cookie (PJSESSIONID / SSCSID / WMONID) 발급.
    http_get_with_retry(session, "https://www.vworld.kr/v4po_main.do", timeout=30)
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
    r = http_post_with_retry(session, LOGIN_URL, data=data, headers=headers, timeout=30)
    body = r.json()
    rm = body.get("resultMap", {})
    if rm.get("result") != "success":
        sys.stderr.write(f"login failed: {rm.get('msg', body)}\n")
        sys.exit(2)
    print(f"[login] {rm.get('usrNam', '?')} 로그인 OK", flush=True)


def fetch_file_list(session: cffi.Session, ds_id: str) -> tuple[list[FileEntry], str]:
    """dtmk 페이지 GET → onclick 의 listFnc.download(dsId, fileNo, fileSize) 추출.

    Round 3 P0:
    - tenacity retry wrap.
    - **raw HTML 반환** — 호출자가 audit trail 로 R2 박제 (페이지 변경 시 진단).
    """
    url = DTMK_URL.format(ds_id=ds_id)
    r = http_get_with_retry(session, url, timeout=30)
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
    return entries, html


def filename_from_disposition(header: str | None) -> str | None:
    """Content-Disposition: attachment; filename=LSMD_CONT_LDREG_충북_충주시.zip; → 그 이름.

    URL-encoded 파일명 (예: `%EC%B6%A9%EB%B6%81`) 자동 decode.
    Round 3 P0 — `except Exception` 제거. urllib unquote 는 입력이 str 이면 항상 성공.
    """
    if not header:
        return None
    m = re.search(r"filename=([^;]+)", header)
    if not m:
        return None
    raw = m.group(1).strip().strip('"')
    # unquote 는 invalid percent-encoding 시 원문 그대로 반환 — 예외 안 던짐.
    return unquote(raw)


def sigungu_from_filename(name: str) -> str:
    """`LSMD_CONT_LDREG_충북_충주시.zip` → `충북_충주시`."""
    m = re.match(r"LSMD_CONT_LDREG_(.+?)\.zip", name)
    return m.group(1) if m else name.removesuffix(".zip")


# ===== R2 client =====
def make_r2() -> Any:
    # boto3.client 는 stubs 가 없어 Any return 이 자연. 격리 service 의 표준 pattern.
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


def r2_head(r2: Any, bucket: str, key: str) -> dict[str, Any] | None:
    """R2 의 object 메타 조회. NoSuchKey/404 = `None`. 다른 에러 = 그대로 raise.

    Round 3 P0 (Codex audit `dtmk_vworld.py:190` finding):
    - 이전 path 가 `except Exception` 으로 권한/네트워크/NoSuchKey 모두 흡수 → silent
      drift 위험 (예: invalid creds 가 idempotent skip 으로 위장).
    - 새 path: NoSuchKey/404 만 None 으로 흡수, 그 외 모든 ClientError 는 raise.
    """
    # NoSuchKey 의 표준 wire 표현 — code 가 "404" string 또는 "NoSuchKey", 또는 HTTP 404.
    not_found_codes = ("404", "NoSuchKey")
    not_found_status = 404
    try:
        return r2.head_object(Bucket=bucket, Key=key)
    except ClientError as e:
        # botocore 의 ClientError 는 dict response 박제. 404 / NoSuchKey 만 expected miss.
        code = e.response.get("Error", {}).get("Code")
        status = e.response.get("ResponseMetadata", {}).get("HTTPStatusCode")
        if code in not_found_codes or status == not_found_status:
            return None
        raise


# ===== main =====
def main() -> int:
    # **SSOT** — Round 3 P0 fix: hardcode 제거. workflow 가 Rust config crate 의
    # `sp9-config-print key dtmk_ds_id` 출력을 `DTMK_DS_ID` env 로 inject. dev local 은
    # 사용자가 명시 set 또는 *명시 default* 를 주석으로 박제 (silent drift 차단).
    # `crates/sp9-base-layer-config/src/lib.rs:54` 의 `DTMK_DS_ID` const 가 단일 출처.
    ds_id = required("DTMK_DS_ID")
    if not ds_id.isdigit():
        sys.stderr.write(
            f"ERROR: DTMK_DS_ID must be numeric, got {ds_id!r}. "
            "workflow가 cargo run -p sp9-base-layer-config --bin sp9-config-print "
            "key dtmk_ds_id 출력을 정확히 inject하는지 확인.\n"
        )
        return 2
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
    try:
        entries, raw_html = fetch_file_list(session, ds_id)
    except (cffi.RequestsError, RetryError) as e:
        sys.stderr.write(f"ERROR: dtmk file list fetch 실패 (3 시도 모두 실패): {e}\n")
        return 2
    print(
        f"[probe] 총 {len(entries)} files (size KB sum = {sum(e.file_size for e in entries):,})",
        flush=True,
    )

    r2 = make_r2()

    # Round 3 (Codex stop-time review): audit trail 은 *guarantee* — best-effort 0.
    # 페이지 변경 / regex miss 시 raw HTML 이 R2 에 박제 되어야 사후 진단 가능.
    # audit PUT 실패 = main 작업 abort (audit 없는 Bronze 빌드는 SSS 불가).
    # path: bronze/<batch>/parcel-dtmk-<ds_id>/audit/<ISO-date>-list.html
    iso_date = time.strftime("%Y-%m-%dT%H-%M-%SZ", time.gmtime())
    audit_key = f"{bronze_prefix}/{batch}/parcel-dtmk-{ds_id}/audit/{iso_date}-list.html"
    try:
        r2_put_with_retry(
            r2,
            bucket=bucket,
            key=audit_key,
            body=raw_html.encode("utf-8"),
            content_type="text/html; charset=utf-8",
            cache_control="public, max-age=31536000, immutable",
            metadata={
                "ds_id": ds_id,
                "fetched_at": iso_date,
                "entry_count": str(len(entries)),
            },
        )
        print(f"[audit] raw HTML 박제 → s3://{bucket}/{audit_key}", flush=True)
    except (ClientError, *transport_exception_types_main, RetryError) as e:
        sys.stderr.write(
            f"FATAL: audit HTML PUT 실패 (3 시도 후) — Bronze 빌드 abort: "
            f"{type(e).__name__}: {e}\n"
        )
        return 2

    if not entries:
        sys.stderr.write("ERROR: file list 비어있음 (페이지 변경 의심) — audit/ 박제됨, abort\n")
        return 2

    def upload_one(idx: int, entry: FileEntry) -> tuple[str, bool, int]:
        """단일 파일 다운 + R2 PUT. (sigungu, downloaded?, bytes).

        Round 3 (Codex stop-time review) — download stream + PUT 둘 다 explicit retry:
        - V-World GET stream 은 connection drop 흔함. tenacity Retrying 으로 *fresh
          attempt* 마다 새 GET → partial bytes 누락 차단.
        - boto3 upload_fileobj 는 자체 multipart retry (max_attempts=5) 보유. 추가
          명시 wrap 은 over-engineering — boto3 fail = 5x 모두 fail = systemic.
        - 호출자 (`as_completed` loop) 가 typed exception 분류 + stderr 박제.
        """
        url = DOWNLOAD_URL.format(ds_id=entry.ds_id, file_no=entry.file_no)
        key_prefix = f"{bronze_prefix}/{batch}/parcel-dtmk-{ds_id}"

        # tenacity-wrapped download attempt loop. 매 attempt 마다 fresh GET (헤더 + body).
        # connection drop 시 partial bytes 안 쓰고 새로 시작 — 안전한 재시도.
        # Round 4 #4 — `is_transient_for_retry` 공유 (V-World cffi.HTTPError 4xx 즉시
        # raise, ClientError 5xx 만 retry, transport 무조건 retry). 이전엔 4xx 도 retry.
        for attempt in Retrying(
            stop=stop_after_attempt(3),
            wait=wait_exponential(multiplier=1, min=2, max=30),
            retry=retry_if_exception(is_transient_for_retry),
            reraise=True,
        ):
            with attempt:
                r = session.get(url, stream=True, timeout=600)
                r.raise_for_status()
                disp = r.headers.get("Content-Disposition")
                filename = filename_from_disposition(disp) or f"file-{entry.file_no}.zip"
                sigungu = sigungu_from_filename(filename)
                content_len = int(r.headers.get("Content-Length", "0"))
                key = f"{key_prefix}/{filename}"

                # idempotent — 이미 같은 size 의 object 가 R2 에 있으면 skip.
                existing = r2_head(r2, bucket, key)
                if (
                    existing
                    and int(existing.get("ContentLength", 0)) == content_len
                    and content_len > 0
                ):
                    r.close()
                    print(
                        f"[{idx:3d}/{len(entries)}] skip {sigungu} ({content_len:,}B, R2 동일)",
                        flush=True,
                    )
                    return sigungu, False, 0

                # streaming PUT — boto3 가 multipart 자동 + 자체 retry (max 5).
                # `_IterStream` 은 stream position rewind 못 함 — 본 attempt 가 fail 시
                # 다음 attempt 가 새 GET 으로 stream 다시 받음.
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
                                "fetched_at": time.strftime(
                                    "%Y-%m-%dT%H:%M:%SZ", time.gmtime()
                                ),
                            },
                        },
                    )
                finally:
                    r.close()
                print(
                    f"[{idx:3d}/{len(entries)}] PUT {sigungu} "
                    f"({content_len:,}B -> s3://{bucket}/{key})",
                    flush=True,
                )
                return sigungu, True, content_len
        # tenacity reraise=True 가 보장 — 미도달 path.
        msg = "tenacity must reraise on retry exhaustion"
        raise RuntimeError(msg)  # pragma: no cover

    # concurrent download (V-World 서버 부담 고려 default 3 parallel).
    # Round 3 P0 — typed exception 분류. silent flatten 대신 (cffi/ClientError) 와 그 외
    # 분리해서 stderr 로깅. exit 1 (failed > 0) 가 workflow 측 alert 트리거.
    started = time.time()
    results: list[tuple[str, bool, int]] = []
    with ThreadPoolExecutor(max_workers=parallel) as ex:
        futs = {ex.submit(upload_one, i + 1, e): e for i, e in enumerate(entries)}
        for fut in as_completed(futs):
            entry = futs[fut]
            try:
                results.append(fut.result())
            except cffi.RequestsError as e:
                sys.stderr.write(
                    f"FAIL fileNo={entry.file_no}: V-World HTTP error: {e}\n"
                )
                results.append((f"fileNo-{entry.file_no}", False, -1))
            except ClientError as e:
                code = e.response.get("Error", {}).get("Code", "?")
                sys.stderr.write(
                    f"FAIL fileNo={entry.file_no}: R2 ClientError [{code}]: {e}\n"
                )
                results.append((f"fileNo-{entry.file_no}", False, -1))
            except transport_exception_types_main as e:
                # botocore transport-level — TCP/TLS/DNS 실패. tenacity 가 3 시도 후 reraise.
                sys.stderr.write(
                    f"FAIL fileNo={entry.file_no}: R2 transport error "
                    f"({type(e).__name__}): {e}\n"
                )
                results.append((f"fileNo-{entry.file_no}", False, -1))
            except (OSError, ConnectionError, TimeoutError) as e:
                sys.stderr.write(
                    f"FAIL fileNo={entry.file_no}: I/O error: {type(e).__name__}: {e}\n"
                )
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
