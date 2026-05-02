# services/api

공짱 HTTP API 서버 (Axum). Walking Skeleton 단계예요.

## Walking Skeleton 범위 (T3)

3 endpoint만 노출해요:

- `GET /healthz` — liveness probe (DB 미접속)
- `POST /users` — `User` 생성 (`PgUserRepository::save`)
- `GET /users/:id` — `User` 조회 (`PgUserRepository::find_by_id`)

## 로컬 실행

```bash
export DATABASE_URL=postgres://user:pass@localhost:5432/gongzzang
cargo run --package api  # api listening on 0.0.0.0:8080

curl -X POST http://localhost:8080/users -H 'content-type: application/json' \
  -d '{"zitadel_sub":"sub-1","email":"a@b.com","display_name":"Alice","user_kind":"individual"}'
curl http://localhost:8080/users/usr_01HXY3NK0Z9F6S1B2C3D4E5F6G
```

## 향후 추가 (sub-project 3 / 5 / 7)

- Zitadel JWT 인증, RBAC
- 전체 `*Repository` (Listing, Parcel ...)
- OpenTelemetry, RFC 9457 Problem Details, utoipa OpenAPI
