-- SP7-iii: 정부 API drift 자동 검출 시스템 SSOT.
-- 모든 cron run + 수동 trigger 결과 영구 보존.

CREATE TABLE api_health_check (
    id BIGSERIAL PRIMARY KEY,
    api_name VARCHAR(64) NOT NULL,
    -- 'data_go_kr.getBrTitleInfo' / 'vworld.getFeature' / etc

    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    status VARCHAR(32) NOT NULL CHECK (status IN (
        'success',
        'http_5xx',
        'http_4xx',
        'parse_fail',
        'timeout',
        'connection_fail'
    )),

    http_code SMALLINT,
    -- nullable (timeout / connection_fail = NULL)

    error_detail TEXT,
    -- masked log (secrets redacted)

    cron_run BOOLEAN NOT NULL,
    -- true = scheduled cron, false = workflow_dispatch (수동 trigger)

    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0)
);

CREATE INDEX idx_api_health_check_api_name_checked_at
    ON api_health_check (api_name, checked_at DESC);

CREATE INDEX idx_api_health_check_failures
    ON api_health_check (api_name, checked_at DESC)
    WHERE status != 'success';

COMMENT ON TABLE api_health_check IS
    '정부 API drift 검출 (SP7-iii). 모든 cron / 수동 trigger 결과 영구 record. SSS SSOT.';

COMMENT ON COLUMN api_health_check.api_name IS
    '대상 API endpoint 식별자. 예: data_go_kr.getBrTitleInfo';

COMMENT ON COLUMN api_health_check.cron_run IS
    'true=schedule cron 자동 실행, false=workflow_dispatch 수동 trigger';
