# @gongzzang/types

여러 패키지가 공유하는 순수 타입 정의.

## 정책

- 런타임 의존성 0 (타입만)
- 도메인 객체는 `@gongzzang/core`가 소유, 여기는 *공유 인프라 타입*만
- 외부 API raw 타입은 `@gongzzang/data-clients`가 소유

## 카테고리 (계획)

```
src/
├── srid.ts            ← SRID 리터럴 유니언 (4326 | 5179 | ...)
├── pnu.ts             ← PNU 브랜드 타입
├── pagination.ts
├── result.ts          ← Result<T, E> 패턴
└── audit.ts           ← AuditLog 공통 인터페이스
```
