# reference/

외부 오픈소스 코드 학습 자료. **빌드 산출물에 포함되지 않음** (.gitignore).

## 권장 clone (개발자가 직접)

```bash
# 도메인 지식 학습용 — MIT 라이선스
git clone https://github.com/UrbanWatcherKr/korean-land-mcp reference/korean-land-mcp
git clone https://github.com/chrisryugj/korean-law-mcp reference/korean-law-mcp
git clone https://github.com/ceami/opendata-mcp reference/opendata-mcp
```

## 활용 방식

- V-World 레이어 ID·코드 매핑 표 학습
- 법제처 API 호출 패턴 학습
- 응답 파싱·에러 처리 reference

## 금지

- ❌ `packages/`, `apps/`에서 reference/를 import 하지 말 것
- ❌ reference 코드를 그대로 복사하지 말 것 (저작권)
- ✅ 학습 후 우리 도메인 모델로 직접 재작성
- ✅ MIT 고지가 필요한 경우 `docs/compliance/third-party-licenses.md` 등록
