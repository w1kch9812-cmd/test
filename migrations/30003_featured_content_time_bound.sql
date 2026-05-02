-- V003_03: featured_content ends_at > starts_at invariant
-- spec § 5.5 누락 invariant 보강 (sub-project 2a-fixup)
--
-- 광고 슬롯의 시작·종료 시각이 뒤집히면 영원히 비활성. CHECK로 차단.

alter table featured_content
    add constraint featured_content_time_bound_chk
    check (ends_at > starts_at);
