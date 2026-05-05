-- V003_08: SP6-i Auth Core schema 자리.
-- users.ci 는 SP6-CI (KISA 본인확인) 가 채움.
-- external_account 의 kakao/naver/google 행은 SP6-Social federation 이 채움.

ALTER TABLE "user" ADD COLUMN ci VARCHAR(88) UNIQUE NULL;
COMMENT ON COLUMN "user".ci IS
  'KISA Connecting Information (88-char hash). NULL until SP6-CI verifies via NICE/Toss/PASS.';

CREATE TABLE external_account (
    id           CHAR(30) PRIMARY KEY,
    user_id      CHAR(30) NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    provider     VARCHAR(32) NOT NULL,
    external_id  VARCHAR(255) NOT NULL,
    linked_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, external_id)
);

CREATE INDEX external_account_user_idx ON external_account(user_id);
CREATE INDEX external_account_provider_idx ON external_account(provider, linked_at DESC);

COMMENT ON TABLE external_account IS
  'Multi-IdP linking. SP6-i populates only zitadel rows on first sign-in. SP6-Social federation populates kakao/naver/google.';

ALTER TABLE external_account
  ADD CONSTRAINT external_account_provider_chk
  CHECK (provider IN ('zitadel', 'kakao', 'naver', 'google', 'apple'));
