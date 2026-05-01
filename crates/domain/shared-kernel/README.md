# crates/domain/shared-kernel

모든 BC가 공유하는 값 객체 (Newtype) + 공통 타입.

## 제공 타입
- **Pnu** — 19자리 필지고유번호 (`Pnu::try_new(s)` 검증)
- **BusinessNumber** — 10자리 사업자번호 (XXX-XX-XXXXX)
- **BrokerLicense** — 공인중개사 자격번호
- **Money** — KRW만 허용, 양수 검증
- **Area** — ㎡ 양수, 평 변환 메서드
- **Geometry** — PostGIS 호환 (SRID 강제)
- **Email**, **PhoneKr** — 형식 검증
- **Ulid** + 도메인 prefix 헬퍼
- **Timestamp** — UTC 강제 + KST 변환

## 정책
- 외부 의존 *최소* (serde, thiserror, ulid, chrono 정도)
- 모든 검증 *생성자에서* (잘못된 값은 Result로 거부)
- 의존성 방향: 모든 도메인 BC → shared-kernel (역방향 X)

## 핵심 패턴
```rust
pub struct Pnu(String);
impl Pnu {
    pub fn try_new(s: &str) -> Result<Self, PnuError> { ... }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

→ → @docs/conventions/rust.md (값 객체 패턴)
