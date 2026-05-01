# @gongzzang/config

공유 빌드/린트/타입 설정. 다른 패키지가 import해서 확장.

## 제공 (계획)

```
src/
├── tsconfig/
│   ├── base.json
│   ├── nextjs.json
│   ├── node.json
│   └── library.json
├── biome/
│   └── extends.json
└── tailwind/
    └── preset.ts
```

## 사용 예시

`apps/web/tsconfig.json`:
```json
{
  "extends": "@gongzzang/config/tsconfig/nextjs.json",
  "include": ["src/**/*", "next-env.d.ts"]
}
```
