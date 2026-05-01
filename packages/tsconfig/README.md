# packages/tsconfig

공유 TypeScript 설정.

## 제공 (sub-project 5+)
- `tsconfig/base.json` — `tsconfig.base.json` 확장 (strict 기본)
- `tsconfig/nextjs.json` — Next.js 16 + React 19
- `tsconfig/library.json` — 라이브러리 빌드 (declaration + composite)
- `tsconfig/node.json` — Node.js 20 (도구·스크립트)

## 사용 예시
```json
// apps/platform-web/tsconfig.json
{
  "extends": "@gongzzang/tsconfig/nextjs.json",
  "include": ["src/**/*", "next-env.d.ts"]
}
```

## 정책
- `strict: true` 모든 변형
- `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` 강제
- `verbatimModuleSyntax` (Biome 호환)
- 변경은 PR에서 모든 워크스페이스 멤버 영향 검토 (CODEOWNERS 알림)

→ → @docs/conventions/typescript.md
