# Sub-project 6-ii Listing Search - Part 03D: Pretendard Self Host

Parent index: [Sub-project 6-ii Listing Search - Part 03](./2026-05-05-sub-project-6-ii-listing-search.part-03.md).
## Task 7: Pretendard self-host + dark mode + CSP cdn 제거

**Files:**
- Create: `apps/web/public/fonts/Pretendard-Regular.woff2`
- Create: `apps/web/public/fonts/Pretendard-Medium.woff2`
- Create: `apps/web/public/fonts/Pretendard-Bold.woff2`
- Create: `apps/web/public/fonts/Pretendard-Heavy.woff2`
- Modify: `apps/web/app/layout.tsx`
- Modify: `packages/ui/tokens/typography.css`
- Modify: `apps/web/proxy.ts` (CSP)

- [ ] **Step 7.1: Pretendard variable woff2 다운로드 (4 가중치)**

```bash
mkdir -p apps/web/public/fonts
cd apps/web/public/fonts
# Pretendard variable subset web font
curl -L -o Pretendard-Regular.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Regular.woff2
curl -L -o Pretendard-Medium.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Medium.woff2
curl -L -o Pretendard-Bold.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Bold.woff2
curl -L -o Pretendard-ExtraBold.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-ExtraBold.woff2
ls -la
```

(NOTE: 4 file ≈ 800 KB 합. license = OFL 1.1 Pretendard 의 라이선스. README 의 attribution 추가 권장.)

- [ ] **Step 7.2: app/layout.tsx 의 next/font/local**

`apps/web/app/layout.tsx` 수정:

```typescript
import localFont from "next/font/local";

const pretendard = localFont({
  src: [
    { path: "../public/fonts/Pretendard-Regular.woff2", weight: "400", style: "normal" },
    { path: "../public/fonts/Pretendard-Medium.woff2", weight: "500", style: "normal" },
    { path: "../public/fonts/Pretendard-Bold.woff2", weight: "700", style: "normal" },
    { path: "../public/fonts/Pretendard-ExtraBold.woff2", weight: "800", style: "normal" },
  ],
  variable: "--font-pretendard",
  display: "swap",
});

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="ko" className={pretendard.variable}>
      <body className="font-sans">
        {/* ... existing providers ... */}
      </body>
    </html>
  );
}
```

(NOTE: 기존 layout.tsx 의 정확한 내용은 Read 후 정확히 변경. providers wrapper, NextIntlClientProvider 등 유지.)

- [ ] **Step 7.3: tokens/typography.css 정리**

`packages/ui/tokens/typography.css` 의 `@import url('https://cdn.jsdelivr.net/...')` 줄 제거. font-family 만 유지:

```css
:root {
  --font-sans: var(--font-pretendard), -apple-system, BlinkMacSystemFont, "Segoe UI",
    "Helvetica Neue", "Apple SD Gothic Neo", sans-serif;
}
```

`tailwind.config.ts` (또는 inline) 의 fontFamily.sans 가 `var(--font-sans)` 사용하도록 — 이미 그럴 수도 있음 (Read 로 확인).

- [ ] **Step 7.4: proxy.ts CSP 의 cdn.jsdelivr 제거**

`apps/web/proxy.ts` 의 CSP `style-src` 정리:

```typescript
const cspHeader = [
  `default-src 'self'`,
  `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'`,
  `style-src 'self' 'unsafe-inline'`,           // cdn.jsdelivr 제거됨
  `img-src 'self' data: blob:`,
  `font-src 'self' data:`,                      // self-host 만 허용
  `connect-src 'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER}`,
  `frame-ancestors 'none'`,
  `base-uri 'self'`,
  `form-action 'self' ${env.ZITADEL_ISSUER}`,
].join("; ");
```

(NOTE: 기존 cdn.jsdelivr.net allow 는 SP6-foundation 시점 자리. self-host 전환 후 삭제. — 단 기존 코드에 이미 추가됐는지 Read 로 확인. 없을 수도 있음.)

- [ ] **Step 7.5: 로컬 검증**

```bash
pnpm --filter=@gongzzang/web dev
# 브라우저: http://localhost:3000/listings
# DevTools → Network → fonts/* 의 200 + 자체 도메인 확인
# DevTools → Console 에 "Refused to execute" / "violates CSP" 경고 없어야
```

- [ ] **Step 7.6: bundle size**

```bash
pnpm --filter=@gongzzang/web test:bundle
```

Expected: under threshold (Pretendard 800 KB → next/font 가 subset 자동 적용 → 실제 < 200KB 추가).

- [ ] **Step 7.7: Commit**

```bash
git add apps/web/public/fonts/ apps/web/app/layout.tsx packages/ui/tokens/typography.css apps/web/proxy.ts
git commit -m "feat(6ii-T7): Pretendard self-host (next/font/local) + CSP cdn.jsdelivr 제거

- public/fonts/Pretendard-{Regular,Medium,Bold,ExtraBold}.woff2 (OFL 1.1, attribution in README)
- app/layout.tsx: localFont (4 weights, swap display, --font-pretendard variable)
- tokens/typography.css: cdn.jsdelivr import 제거, --font-sans = var(--font-pretendard) chain
- proxy.ts CSP: style-src 의 cdn.jsdelivr.net 제거 (self-host 전환 완료)"
```

---
