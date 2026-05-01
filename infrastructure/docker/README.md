# 로컬 Docker Compose 인프라

로컬 개발용 PostgreSQL 17 + PostGIS 3.5 + Valkey 8 컨테이너입니다.

## 사전 준비

- Docker Desktop이 설치돼 있어야 해요. (`docker --version`으로 확인)
- `.env` 파일을 만들어 주세요. `.env.example`을 복사하면 돼요.

```bash
cp infrastructure/docker/.env.example infrastructure/docker/.env
```

## 기동

```bash
docker compose -f infrastructure/docker/docker-compose.yml --env-file infrastructure/docker/.env up -d
```

## 중지

```bash
docker compose -f infrastructure/docker/docker-compose.yml down
```

## 데이터 초기화 (볼륨까지 삭제)

```bash
docker compose -f infrastructure/docker/docker-compose.yml down -v
```

## 접속

```bash
docker exec -it gongzzang-postgres psql -U gongzzang
```

## 헬스체크

```bash
docker compose -f infrastructure/docker/docker-compose.yml ps
```

`postgres`, `valkey` 두 서비스 모두 `healthy`로 보이면 정상이에요.

## 주의 사항

- `.env`는 `.gitignore`에 포함돼 있어요. 절대 커밋하지 마세요.
- `.env.example`의 비밀번호(`changeme_local_only`)는 로컬 전용이에요. **운영 환경에서 절대 사용하지 마세요.**
- 운영 환경은 추후 AWS RDS + ElastiCache로 교체될 예정이에요. 본 컴포즈는 로컬 개발 한정이에요.
