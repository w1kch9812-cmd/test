-- V002_02: audit_log immutable 트리거 — UPDATE/DELETE 박탈 (defense-in-depth)
-- writer role의 GRANT/REVOKE가 무력화되는 경우(슈퍼유저, 스키마 변경)에도
-- DB 레벨에서 무결성 보장. audit_archiver만 DELETE 가능 (retention 1년 후 R2 archive).

create or replace function reject_audit_mutation() returns trigger
language plpgsql
as $$
begin
    raise exception 'audit_log is immutable: % not allowed (current_user=%, only gongzzang_audit_archiver may DELETE after retention)',
        tg_op, current_user;
end $$;

create trigger trg_audit_no_update
    before update on audit_log
    for each row
    when (current_user <> 'gongzzang_audit_archiver')
    execute function reject_audit_mutation();

create trigger trg_audit_no_delete
    before delete on audit_log
    for each row
    when (current_user <> 'gongzzang_audit_archiver')
    execute function reject_audit_mutation();
