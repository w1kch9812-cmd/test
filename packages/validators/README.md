# @gongzzang/validators

Zod 스키마 모음. 입력 검증·DTO 정의·타입 추론 단일 소스.

## 정책

- 모든 Server Action 입력 검증
- 외부 API 응답 검증 (자체 안전망)
- DB Insert/Update DTO도 여기서 정의
- `z.infer`로 타입 추출, `packages/types`와 충돌 없음

## 카테고리 (계획)

```
src/
├── parcel.ts          ← PNU, 주소 검증
├── geo.ts             ← 좌표, SRID 검증
├── law.ts             ← 법령 검색 입력
├── auth.ts
└── shared.ts          ← 페이지네이션 등
```
