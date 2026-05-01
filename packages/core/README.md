# @gongzzang/core

도메인 핵심 — Clean Architecture의 코어 레이어.

## 책임

- **도메인 모델**: `Parcel`, `Zoning`, `Building`, `Law`, `Ordinance` 등 순수 객체
- **Use Cases**: `AnalyzeParcelUseCase`, `SearchLawUseCase` 등
- **Port (인터페이스)**: `LandInfoProvider`, `LegalInfoProvider`, `OpenDataProvider`
- **도메인 서비스**: 여러 모델 간 비즈니스 로직

## 의존하지 않는 것

- 외부 API SDK
- HTTP 라이브러리
- DB 라이브러리
- React, Next.js
- Node.js 특정 모듈

→ **순수 TypeScript만**. Adapter는 `packages/data-clients`, `packages/db`에서 구현.

## 디렉토리 (TODO)

```
src/
├── parcel/
│   ├── parcel.entity.ts
│   ├── analyze-parcel.usecase.ts
│   └── ports/land-info-provider.port.ts
├── law/
├── building/
└── shared/
```
