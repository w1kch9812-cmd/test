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
from botocore.exceptions import ClientError

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
