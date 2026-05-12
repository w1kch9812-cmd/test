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
from curl_cffi.requests import exceptions as cffi_exc

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


# Round 4 #3 + #4 — `is_transient_for_retry` 회귀 (V-World cffi.HTTPError 4xx vs 5xx
# + connection-level transport + R2 ClientError + transport_exception_types_main).

def test_is_transient_cffi_http_4xx_returns_false() -> None:
    """V-World 4xx (인증 만료 / dataset 부재) — 즉시 raise, retry 무의미."""
    response = MagicMock()
    response.status_code = 401
    err = cffi_exc.HTTPError("unauthorized")
    err.response = response
    assert not dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_cffi_http_5xx_returns_true() -> None:
    """V-World 5xx (server fault) — retry 대상."""
    response = MagicMock()
    response.status_code = 502
    err = cffi_exc.HTTPError("bad gateway")
    err.response = response
    assert dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_cffi_no_response_returns_true() -> None:
    """response 박제 0 = connection-level fail = retry."""
    err = cffi_exc.HTTPError("no response")
    assert dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_cffi_connection_error_returns_true() -> None:
    """V-World cffi.ConnectionError — TCP 단계 fail, retry."""
    err = cffi_exc.ConnectionError("tcp reset")
    assert dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_botocore_5xx_returns_true() -> None:
    """ClientError 5xx — retry."""
    err = ClientError(
        {"Error": {"Code": "InternalError"},
         "ResponseMetadata": {"HTTPStatusCode": 500}},
        "PutObject",
    )
    assert dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_botocore_4xx_returns_false() -> None:
    """ClientError 4xx (AccessDenied) — immediate raise."""
    err = ClientError(
        {"Error": {"Code": "AccessDenied"},
         "ResponseMetadata": {"HTTPStatusCode": 403}},
        "PutObject",
    )
    assert not dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_botocore_throttling_returns_true() -> None:
    """ClientError Throttling — retry (rate limit, 백오프 후 OK)."""
    err = ClientError(
        {"Error": {"Code": "Throttling"},
         "ResponseMetadata": {"HTTPStatusCode": 429}},
        "PutObject",
    )
    assert dtmk_vworld.is_transient_for_retry(err)


def test_is_transient_unknown_exception_returns_false() -> None:
    """예상 못 한 exception (programming error) — retry 안 함."""
    assert not dtmk_vworld.is_transient_for_retry(ValueError("unexpected"))
    assert not dtmk_vworld.is_transient_for_retry(KeyError("missing"))


# Round 5+ — ADR 0029 namespace 회귀 test.


def _clear_r2_env(monkeypatch: pytest.MonkeyPatch) -> None:
    """모든 R2 관련 env 를 dict 에서 제거 (test 격리)."""
    keys_to_clear = [
        "ETL_ENVIRONMENT",
        "R2_ACCOUNT_ID",
        "R2_ACCESS_KEY",
        "R2_SECRET_KEY",
        "R2_BUCKET",
    ]
    for env in ("LOCAL", "STAGING", "PRODUCTION"):
        for suffix in ("ACCOUNT_ID", "ACCESS_KEY", "SECRET_KEY", "BUCKET"):
            keys_to_clear.append(f"R2_{env}_{suffix}")
    for k in keys_to_clear:
        monkeypatch.delitem(dtmk_vworld.ENV, k, raising=False)


def _set_full_namespace(
    monkeypatch: pytest.MonkeyPatch, env: str, marker: str
) -> None:
    """4 suffix 모두 set — atomic loader 의 정상 path 시나리오."""
    prefix = f"R2_{env.upper()}_"
    for s in ("ACCOUNT_ID", "ACCESS_KEY", "SECRET_KEY", "BUCKET"):
        monkeypatch.setitem(dtmk_vworld.ENV, f"{prefix}{s}", f"{marker}-{s.lower()}")


def test_load_r2_credentials_full_namespace(monkeypatch: pytest.MonkeyPatch) -> None:
    """production + 4 suffix 모두 set → atomic 통과, 모두 production 자격."""
    _clear_r2_env(monkeypatch)
    monkeypatch.setitem(dtmk_vworld.ENV, "ETL_ENVIRONMENT", "production")
    _set_full_namespace(monkeypatch, "PRODUCTION", "prod")
    # legacy 도 set 했지만 namespace 우선 (mix 0).
    monkeypatch.setitem(dtmk_vworld.ENV, "R2_ACCOUNT_ID", "leak-legacy")
    creds = dtmk_vworld.load_r2_credentials()
    assert creds["ACCOUNT_ID"] == "prod-account_id"
    assert creds["ACCESS_KEY"] == "prod-access_key"
    assert creds["SECRET_KEY"] == "prod-secret_key"
    assert creds["BUCKET"] == "prod-bucket"


def test_load_r2_credentials_partial_namespace_fails_fast(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """ADR 0035 핵심 — 부분 namespace = credential mix 차단 fail-fast.

    이전 Codex stop-hook 발견 시나리오 회귀.
    """
    _clear_r2_env(monkeypatch)
    monkeypatch.setitem(dtmk_vworld.ENV, "ETL_ENVIRONMENT", "production")
    # ACCOUNT_ID 만 namespace set, 나머지 3개 unset.
    monkeypatch.setitem(
        dtmk_vworld.ENV, "R2_PRODUCTION_ACCOUNT_ID", "prod-acct"
    )

    with pytest.raises(SystemExit) as exc_info:
        dtmk_vworld.load_r2_credentials()
    assert exc_info.value.code == 2
    err = capsys.readouterr().err
    assert "partial" in err.lower()
    assert "ACCESS_KEY" in err  # 누락 항목 박제


def test_load_r2_credentials_legacy_completely_ignored(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """ADR 0035 — legacy `R2_*` (namespace 없음) **완전 제거**. 어떤 env 에서도 활성 X.

    staging + legacy 4개 모두 set 돼있어도 namespace 0 = `None` (R2 비활성).
    이전 (ADR 0029 1-sprint backward-compat) 자체가 trick — 본 test 가 그 회귀 invariant.
    """
    _clear_r2_env(monkeypatch)
    monkeypatch.setitem(dtmk_vworld.ENV, "ETL_ENVIRONMENT", "staging")
    monkeypatch.setitem(dtmk_vworld.ENV, "R2_ACCOUNT_ID", "leg-acct")
    monkeypatch.setitem(dtmk_vworld.ENV, "R2_ACCESS_KEY", "leg-key")
    monkeypatch.setitem(dtmk_vworld.ENV, "R2_SECRET_KEY", "leg-secret")
    monkeypatch.setitem(dtmk_vworld.ENV, "R2_BUCKET", "leg-bucket")
    creds = dtmk_vworld.load_r2_credentials()
    assert creds is None, "ADR 0035: legacy R2_* must NOT activate anywhere"


def test_load_r2_credentials_local_namespace_zero_returns_none(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """ADR 0035 — local + namespace 4개 모두 unset = None (R2 비활성, local-only mode).

    이전 (ADR 0029) 의 fail-fast 정책 변경 — local-only mode 가 정상 path.
    """
    _clear_r2_env(monkeypatch)
    monkeypatch.setitem(dtmk_vworld.ENV, "ETL_ENVIRONMENT", "local")
    # legacy 도 0, namespace 도 0.
    assert dtmk_vworld.load_r2_credentials() is None


def test_load_r2_credentials_fail_fast_when_environment_unset(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """ADR 0029 — `ETL_ENVIRONMENT` 미설정 시 fail-fast."""
    _clear_r2_env(monkeypatch)
    with pytest.raises(SystemExit) as exc_info:
        dtmk_vworld.load_r2_credentials()
    assert exc_info.value.code == 2
