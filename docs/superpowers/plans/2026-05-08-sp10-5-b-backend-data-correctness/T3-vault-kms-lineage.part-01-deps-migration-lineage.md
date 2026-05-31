# T3 Vault KMS Lineage - Part 01: Dependencies, Vault Migration, And Lineage

Parent index: [T3 Vault KMS Lineage](./T3-vault-kms-lineage.md).

## Step 3.1: Add aws-sdk-kms + aes-gcm dependencies

- [ ] **Step 3.1.1: Modify `crates/db/Cargo.toml`**

```toml
[dependencies]
# ... 기존 ...
aws-sdk-kms = { workspace = true }
aes-gcm = { workspace = true }
```

Workspace `Cargo.toml` 에 미정의 시 추가:

```toml
[workspace.dependencies]
# ... 기존 ...
aws-sdk-kms = "1"
aes-gcm = "0.10"
```

- [ ] **Step 3.1.2: Build check**

```bash
cargo check -p gongzzang-db
# Expected: Finished — kms / aes-gcm 컴파일 가능
```

- [ ] **Step 3.1.3: Commit**

```bash
git add crates/db/Cargo.toml Cargo.toml
git commit -m "chore(sp10-5-b-T3): add aws-sdk-kms + aes-gcm deps to db crate"
```

---

## Step 3.2: Migration 30013 — pii_vault table + RLS

- [ ] **Step 3.2.1: Create `migrations/30013_pii_vault.sql`**

```sql
-- V003_13: parcel_external_data_pii_vault — KMS envelope encrypted Tier 2 vault.
--
-- Spec SSOT: design.md §6.2.
--
-- ADR 근거: parcel_external_data PK 가 (pnu char(19), source varchar(40))
-- composite. PostgreSQL FK 는 referencing/referenced 컬럼 타입이 *정확히* 일치
-- 해야 함 → vault 의 pnu/source 도 동일 타입 사용. source CHECK 도 fail-safe
-- 로 별도 추가 (parent CHECK 와 sync 깨질 위험 대신 명시적 vault enum).
--
-- Lock safety: 신규 테이블 생성 + RLS policy 추가 — 기존 테이블 lock 영향 없음.

BEGIN;

CREATE TABLE parcel_external_data_pii_vault (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    pnu              char(19)     NOT NULL,
    source           varchar(40)  NOT NULL CHECK (source IN (
        'vworld',                          -- legacy alias (backfill 이전 row 호환)
        'vworld_parcel',
        'data_go_kr_building',
        'data_go_kr_land',
        'data_go_kr_realtransaction',
        'korean_law'
    )),
    ciphertext_blob  BYTEA        NOT NULL,
    kms_key_id       TEXT         NOT NULL,
    encryption_ctx   JSONB        NOT NULL DEFAULT '{}',
    captured_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),
    expires_at       TIMESTAMPTZ  NOT NULL,
    FOREIGN KEY (pnu, source) REFERENCES parcel_external_data(pnu, source) ON DELETE CASCADE
);

CREATE INDEX parcel_external_data_pii_vault_pnu_source_idx
    ON parcel_external_data_pii_vault (pnu, source);

-- Row-Level Security: 기본 차단, 'admin' role 만 접근.
-- Application 은 `SET LOCAL app.role = 'admin'` 트랜잭션 시작 시 명시.
ALTER TABLE parcel_external_data_pii_vault ENABLE ROW LEVEL SECURITY;

CREATE POLICY vault_admin_only ON parcel_external_data_pii_vault
    USING (current_setting('app.role', true) = 'admin');

COMMENT ON TABLE parcel_external_data_pii_vault IS
    'Tier 2 PII vault — KMS envelope encrypted raw responses. Access via /api/admin/raw_vault.';
COMMENT ON COLUMN parcel_external_data_pii_vault.ciphertext_blob IS
    'Format: enc_dek_len(4B BE) || enc_dek || iv(12B) || ciphertext (AES-256-GCM)';
COMMENT ON COLUMN parcel_external_data_pii_vault.encryption_ctx IS
    'AAD for AES-GCM. Includes pnu/source/captured_at for binding.';

COMMIT;
```

- [ ] **Step 3.2.2: Run forward migration**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30013/migrate pii_vault
```

- [ ] **Step 3.2.3: Verify schema**

```bash
psql gongzzang_dev -c "\d parcel_external_data_pii_vault"
# Expected: 표시된 컬럼: id, pnu char(19), source varchar(40), ciphertext_blob bytea, ...
psql gongzzang_dev -c "SELECT relname, relrowsecurity FROM pg_class WHERE relname = 'parcel_external_data_pii_vault';"
# Expected: relrowsecurity = t (RLS enabled)
```

- [ ] **Step 3.2.4: Commit**

```bash
git add migrations/30013_pii_vault.sql
git commit -m "feat(sp10-5-b-T3): migration 30013 — pii_vault table + RLS + composite FK"
```

---

## Step 3.3: Migration 30014 — external_data_lineage columns

- [ ] **Step 3.3.1: Create `migrations/30014_external_data_lineage.sql`**

```sql
-- V003_14: lineage columns on parcel_external_data — data provenance + drift detection.
--
-- Spec SSOT: design.md §6.1.
--
-- 컬럼:
--   license            : 데이터셋 라이선스 (예: 'KOGL-TYPE1', 'V-WORLD-TOS')
--   api_version        : upstream API version (예: 'data.go.kr/BldRgstService_v2')
--   sanitizer_version  : AllowlistSanitizer 버전 (스키마 변경 시 증가)
--   schema_hash        : SHA-256 of allowlist definition (drift detection input)
--
-- Lock safety: ADD COLUMN with DEFAULT (sanitizer_version DEFAULT 1) 는
-- PostgreSQL 11+ 에서 instant operation (rewrite 없음). NULL 허용 컬럼은
-- 추가 lock 없이 즉시 적용.

BEGIN;

ALTER TABLE parcel_external_data
    ADD COLUMN license            TEXT,
    ADD COLUMN api_version        TEXT,
    ADD COLUMN sanitizer_version  INT NOT NULL DEFAULT 1,
    ADD COLUMN schema_hash        TEXT;

-- Backfill 기존 레코드: schema_hash 는 SHA-256 hash 아님을 'legacy:' prefix 로 표시.
UPDATE parcel_external_data
   SET schema_hash       = 'legacy:' || md5(raw_response::text),
       sanitizer_version = 0
 WHERE schema_hash IS NULL OR schema_hash = '';

COMMENT ON COLUMN parcel_external_data.license IS
    'Open data license code (KOGL-TYPE1, V-WORLD-TOS, etc).';
COMMENT ON COLUMN parcel_external_data.sanitizer_version IS
    'AllowlistSanitizer version. 0 = legacy pre-SP10.5-B, 1+ = post-SP10.5-B.';
COMMENT ON COLUMN parcel_external_data.schema_hash IS
    'SHA-256 of allowlist (source:version:sorted_paths). Drift detection input.';

COMMIT;
```

- [ ] **Step 3.3.2: Run forward migration**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30014/migrate external_data_lineage
```

- [ ] **Step 3.3.3: Verify columns + backfill**

```bash
psql gongzzang_dev -c "\d parcel_external_data" | grep -E "license|api_version|sanitizer_version|schema_hash"
# Expected: 4 lineage columns visible
psql gongzzang_dev -c "SELECT sanitizer_version, count(*) FROM parcel_external_data GROUP BY sanitizer_version;"
# Expected: sanitizer_version = 0 for all pre-migration rows (legacy backfill)
psql gongzzang_dev -c "SELECT count(*) FROM parcel_external_data WHERE schema_hash LIKE 'legacy:%';"
# Expected: count > 0 if any pre-existing rows
```

- [ ] **Step 3.3.4: Commit**

```bash
git add migrations/30014_external_data_lineage.sql
git commit -m "feat(sp10-5-b-T3): migration 30014 — lineage cols + legacy backfill"
```

---
