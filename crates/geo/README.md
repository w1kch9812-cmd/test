# crates/geo

좌표계 변환 + 지오메트리 유틸 + Naver Maps 호환.

## 책임
- SRID 변환 (4326 ↔ 5179 ↔ 5186 ↔ 3857)
- PNU 파싱 + 정규화
- 거리·면적 계산 (㎡ ↔ 평)
- 좌표 검증 (한국 영역 내)
- Naver Maps 좌표 호환

## 의존
- `crates/domain/shared-kernel`
- `proj` crate (좌표 변환)
- `geo-types`, `geozero` (지오메트리)

## 정책
- DB 저장: 항상 EPSG:4326
- 거리 연산: 5179 (UTM-K)
- 타일: 3857 (Web Mercator)
- 입출력 시 SRID 명시 강제 (런타임 검증)
- 한국 영역 외 좌표 = 검증 에러
