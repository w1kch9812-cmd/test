"""Round 3 P0 regression tests for `dtmk_vworld.py`.

회귀 트리거:
- DTMK_DS_ID SSOT — env-driven, 하드코딩 제거.
- `r2_head` 가 NoSuchKey 만 None, 그 외 ClientError 는 propagate.
- `filename_from_disposition` 의 URL-decode + None 분기.
- `sigungu_from_filename` 의 LSMD prefix 처리.
"""

from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from botocore.exceptions import (
    ClientError,
    ConnectionClosedError,
    EndpointConnectionError,
    ReadTimeoutError,
)

# parent dir import — pyproject 의 packaging 미설정이라 path 주입.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import dtmk_vworld  # noqa: E402


def test_filename_from_disposition_handles_url_encoded() -> None:
    """%EC%B6%A9%EB%B6%81 같은 한글 URL-encoded 파일명을 자동 decode."""
    header = (
        "attachment; filename=LSMD_CONT_LDREG_"
        "%EC%B6%A9%EB%B6%81_%EC%B6%A9%EC%A3%BC%EC%8B%9C.zip;"
    )
    name = dtmk_vworld.filename_from_disposition(header)
    assert name == "LSMD_CONT_LDREG_충북_충주시.zip"


def test_filename_from_disposition_returns_none_when_no_filename() -> None:
    assert dtmk_vworld.filename_from_disposition(None) is None
    assert dtmk_vworld.filename_from_disposition("") is None
    assert dtmk_vworld.filename_from_disposition("attachment") is None


def test_sigungu_from_filename_strips_lsmd_prefix() -> None:
    assert dtmk_vworld.sigungu_from_filename("LSMD_CONT_LDREG_충북_충주시.zip") == "충북_충주시"
    assert dtmk_vworld.sigungu_from_filename("other.zip") == "other"


def test_r2_head_returns_none_on_no_such_key() -> None:
    """Round 3 P0 — `except Exception` 제거 후 NoSuchKey 만 None 으로 흡수."""
    r2 = MagicMock()
    r2.head_object.side_effect = ClientError(
        {"Error": {"Code": "NoSuchKey", "Message": "not found"},
         "ResponseMetadata": {"HTTPStatusCode": 404}},
        "HeadObject",
    )
    assert dtmk_vworld.r2_head(r2, "bucket", "missing") is None


def test_r2_head_returns_none_on_404_status() -> None:
    """대안 wire — code 가 '404' string 인 경우도 흡수."""
    r2 = MagicMock()
    r2.head_object.side_effect = ClientError(
        {"Error": {"Code": "404", "Message": "Not Found"},
         "ResponseMetadata": {"HTTPStatusCode": 404}},
        "HeadObject",
    )
    assert dtmk_vworld.r2_head(r2, "bucket", "missing") is None


def test_r2_head_propagates_access_denied() -> None:
    """Round 3 P0 — AccessDenied / 5xx 등은 silent 흡수 X, 그대로 propagate."""
    r2 = MagicMock()
    r2.head_object.side_effect = ClientError(
        {"Error": {"Code": "AccessDenied", "Message": "denied"},
         "ResponseMetadata": {"HTTPStatusCode": 403}},
        "HeadObject",
    )
    with pytest.raises(ClientError) as exc_info:
        dtmk_vworld.r2_head(r2, "bucket", "any")
    assert exc_info.value.response["Error"]["Code"] == "AccessDenied"


def test_r2_head_returns_metadata_on_success() -> None:
    r2 = MagicMock()
    r2.head_object.return_value = {"ContentLength": 12345, "ETag": '"abc"'}
    result = dtmk_vworld.r2_head(r2, "bucket", "exists")
    assert result is not None
    assert result["ContentLength"] == 12345


# Round 3 stop-hook fix — audit/retry guarantee 회귀 tests.

def test_r2_put_with_retry_raises_immediately_on_4xx() -> None:
    """4xx (AccessDenied / NoSuchBucket) 는 즉시 raise — retry 무의미."""
    r2 = MagicMock()
    err = ClientError(
        {"Error": {"Code": "AccessDenied", "Message": "denied"},
         "ResponseMetadata": {"HTTPStatusCode": 403}},
        "PutObject",
    )
    r2.put_object.side_effect = err
    with pytest.raises(ClientError) as exc_info:
        dtmk_vworld.r2_put_with_retry(
            r2,
            bucket="b",
            key="k",
            body=b"x",
            content_type="text/plain",
        )
    assert exc_info.value.response["Error"]["Code"] == "AccessDenied"
    # 4xx 는 첫 시도에서만 — retry 안 됨.
    assert r2.put_object.call_count == 1


def test_r2_put_with_retry_retries_on_5xx_then_succeeds() -> None:
    """5xx transient — tenacity 가 1차 fail 후 2차에서 성공."""
    r2 = MagicMock()
    transient_err = ClientError(
        {"Error": {"Code": "InternalError", "Message": "oops"},
         "ResponseMetadata": {"HTTPStatusCode": 500}},
        "PutObject",
    )
    # 1차 fail, 2차 success.
    r2.put_object.side_effect = [transient_err, None]
    dtmk_vworld.r2_put_with_retry(
        r2,
        bucket="b",
        key="k",
        body=b"x",
        content_type="text/plain",
    )
    assert r2.put_object.call_count == 2


def test_r2_put_with_retry_exhausts_then_raises() -> None:
    """3 시도 모두 5xx — 마지막에 ClientError raise."""
    r2 = MagicMock()
    transient_err = ClientError(
        {"Error": {"Code": "ServiceUnavailable", "Message": "down"},
         "ResponseMetadata": {"HTTPStatusCode": 503}},
        "PutObject",
    )
    r2.put_object.side_effect = transient_err
    with pytest.raises(ClientError) as exc_info:
        dtmk_vworld.r2_put_with_retry(
            r2,
            bucket="b",
            key="k",
            body=b"x",
            content_type="text/plain",
        )
    assert exc_info.value.response["Error"]["Code"] == "ServiceUnavailable"
    assert r2.put_object.call_count == 3


def test_r2_put_with_retry_passes_optional_headers() -> None:
    """metadata + cache_control 인자가 boto3 put_object 에 그대로 전달."""
    r2 = MagicMock()
    r2.put_object.return_value = None
    dtmk_vworld.r2_put_with_retry(
        r2,
        bucket="b",
        key="audit/x.html",
        body=b"<html/>",
        content_type="text/html; charset=utf-8",
        cache_control="public, max-age=31536000, immutable",
        metadata={"ds_id": "30563"},
    )
    args = r2.put_object.call_args
    assert args.kwargs["Bucket"] == "b"
    assert args.kwargs["Key"] == "audit/x.html"
    assert args.kwargs["CacheControl"] == "public, max-age=31536000, immutable"
    assert args.kwargs["Metadata"]["ds_id"] == "30563"


# Round 3 stop-hook fix v2 — transport-level retry 회귀.
# Codex finding: `ClientError` 만 retry 했으나 `BotoCoreError` 서브클래스 (transport
# fail) 가 누락되어 R2 endpoint 가 connection drop / TLS handshake fail / DNS unreach
# 인 케이스가 retry 안 됨.

def test_r2_put_with_retry_retries_on_endpoint_connection_error() -> None:
    """DNS / connection refused / endpoint unreachable — transport-level transient."""
    r2 = MagicMock()
    transport_err = EndpointConnectionError(endpoint_url="https://r2.test")
    # 1차 fail (transport), 2차 success.
    r2.put_object.side_effect = [transport_err, None]
    dtmk_vworld.r2_put_with_retry(
        r2,
        bucket="b",
        key="k",
        body=b"x",
        content_type="text/plain",
    )
    assert r2.put_object.call_count == 2


def test_r2_put_with_retry_retries_on_connection_closed_error() -> None:
    """TCP RST / connection closed mid-request — transport-level transient."""
    r2 = MagicMock()
    transport_err = ConnectionClosedError(endpoint_url="https://r2.test")
    r2.put_object.side_effect = [transport_err, None]
    dtmk_vworld.r2_put_with_retry(
        r2,
        bucket="b",
        key="k",
        body=b"x",
        content_type="text/plain",
    )
    assert r2.put_object.call_count == 2


def test_r2_put_with_retry_retries_on_read_timeout() -> None:
    """upstream read timeout — transport-level transient."""
    r2 = MagicMock()
    transport_err = ReadTimeoutError(endpoint_url="https://r2.test")
    # 2회 transient + 3회 success.
    r2.put_object.side_effect = [transport_err, transport_err, None]
    dtmk_vworld.r2_put_with_retry(
        r2,
        bucket="b",
        key="k",
        body=b"x",
        content_type="text/plain",
    )
    assert r2.put_object.call_count == 3


def test_r2_put_with_retry_exhausts_on_persistent_transport_failure() -> None:
    """3회 모두 transport fail → 마지막에 reraise (RetryError 또는 원본 transport)."""
    r2 = MagicMock()
    transport_err = EndpointConnectionError(endpoint_url="https://r2.test")
    r2.put_object.side_effect = transport_err
    with pytest.raises(EndpointConnectionError):
        dtmk_vworld.r2_put_with_retry(
            r2,
            bucket="b",
            key="k",
            body=b"x",
            content_type="text/plain",
        )
    assert r2.put_object.call_count == 3
