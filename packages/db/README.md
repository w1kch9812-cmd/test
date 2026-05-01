# @gongzzang/db

PostgreSQL + PostGIS 스키마, 마이그레이션, Repository 구현.

## 도구

- ORM: **Drizzle ORM** (PostGIS 타입 확장)
- 마이그레이션: **drizzle-kit**
- 공간 확장: **PostGIS**

## 디렉토리 (TODO)

```
src/
├── schema/                      ← 도메인별 분할 (500줄 규칙)
│   ├── auth.ts
│   ├── parcel.ts
│   ├── building.ts
│   ├── law-cache.ts
│   └── audit.ts
├── relations.ts
├── client.ts                    ← drizzle 클라이언트 팩토리
└── repositories/                ← Port 구현
    ├── parcel.repo.ts
    └── ...
drizzle/
├── migrations/                  ← drizzle-kit generate 결과
└── meta/
```

## 정책

- 모든 좌표 컬럼에 **SRID 명시** (`geometry(Polygon, 5179)` 등)
- 공간 인덱스는 **GIST**
- Optimistic Locking (`version` 컬럼)
- Soft Delete (`deleted_at`)
- raw 응답 보존 컬럼 (`raw_response JSONB`)
