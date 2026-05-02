-- V003_02: BVQ + LRQ optimistic locking — concurrent admin edit lost-update 방어
-- spec § 5.5 누락 invariant 보강 (sub-project 2a-fixup)
--
-- 두 어드민이 같은 큐 항목을 동시 검토하면 마지막 쓰기가 이전 쓰기를 덮어씀.
-- version 컬럼으로 OCC 강제: UPDATE ... WHERE id=X AND version=N → version=N+1.

alter table business_verification_queue
    add column version bigint not null default 1;

alter table listing_review_queue
    add column version bigint not null default 1;
