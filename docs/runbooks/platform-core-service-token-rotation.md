# Platform Core Service Token Rotation Runbook

## Scope

This runbook covers the temporary `PLATFORM_CORE_SERVICE_TOKEN` used by
`gongzzang-api` when calling Platform Core Catalog APIs. This token is a
bridge control until mTLS or short-lived workload identity is deployed.
Production should prefer `PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE` whenever
Platform Core or the service mesh can mount a short-lived credential file.

## Production Metadata

Production deployments must configure all of these values:

- `PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE` when available
- `PLATFORM_CORE_SERVICE_TOKEN`
- `PLATFORM_CORE_SERVICE_TOKEN_SCOPE`
- `PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT`
- `PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT`
- `PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER`

Required values:

- Workload identity token files are read before each Platform Core request so
  file rotation can take effect without changing process configuration.
- Scope must be `catalog:read`.
- TTL must be 90 days or lower.
- `issued_at` must not be in the future.
- `expires_at` must not be expired.
- Local `dev-platform-core-service-token-*` values are forbidden in production examples.

## Rotation Procedure

1. Prefer mounting a short-lived workload identity token file and setting
   `PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE`.
2. Create a replacement static token on the Platform Core side with `catalog:read`
   scope.
3. Record the issue timestamp, expiry timestamp, and rotation owner.
4. Update the production secret store with the new token and metadata as one
   change.
5. Deploy Gongzzang API and confirm startup succeeds.
6. Confirm Platform Core receives `x-gongzzang-service-auth-policy-id` and
   `x-gongzzang-service-auth-scope` on Catalog reads.
7. Confirm Platform Core receives `x-gongzzang-service-auth-source:
   gongzzang-api`, `x-gongzzang-service-auth-target: platform-core-api`, and
   `x-gongzzang-allowed-call-id:
   gongzzang_api_to_platform_core_catalog_read` so default-deny authorization
   can match the allowed-call matrix.
8. Revoke the old token after the rollout is healthy.

## Rollback

Rollback is allowed only before old-token revocation. Restore the previous
token and its exact metadata together, then redeploy. Do not extend an expired
token; issue a replacement instead.

## Workload Identity Cutover

The replacement target is SPIFFE/SPIRE or cloud workload identity with mTLS and
default-deny service authorization. Bearer token removal is blocked until the
allowed-call matrix and Platform Core authorization policy both enforce the
`gongzzang_api_to_platform_core_catalog_read` call.
