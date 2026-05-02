-- V002_02: audit_log immutable 트리거 — UPDATE/DELETE 박탈 (defense-in-depth)
-- writer role의 GRANT/REVOKE가 무력화되는 경우(슈퍼유저, 스키마 변경)에도
-- DB 레벨에서 무결성 보장. audit_archiver만 DELETE 가능 (retention 1년 후 R2 archive).
--
-- SQLSTATE '45000' (invalid_authorization_specification) — sqlx::Error::Database::code()로
-- 정확히 매칭 가능. 메시지 i18n 변경에 영향 없음.

create or replace function reject_audit_mutation() returns trigger
language plpgsql
as $$
begin
    raise exception 'audit_log is immutable: % not allowed (current_user=%, only gongzzang_audit_archiver may DELETE after retention)',
        tg_op, current_user
        using errcode = '45000';
end $$;

-- current_user (NOT session_user): SET ROLE 후의 effective role을 검사.
-- 운영 archiver worker는 gongzzang_archiver_user로 로그인 후 SET ROLE gongzzang_audit_archiver
-- 패턴으로 동작 — 이 트리거는 그 경로만 통과시킴.

create or replace trigger trg_audit_no_update
    before update on audit_log
    for each row
    when (current_user <> 'gongzzang_audit_archiver')
    execute function reject_audit_mutation();

create or replace trigger trg_audit_no_delete
    before delete on audit_log
    for each row
    when (current_user <> 'gongzzang_audit_archiver')
    execute function reject_audit_mutation();
