-- V002_01: DB Role 분리 (writer/reader/audit_archiver) — 최소 권한 + audit_log immutable (spec § 6)

-- 권한 분리 (audit immutable + 최소 권한)

-- 1. 일반 앱 writer (INSERT/UPDATE/DELETE 대부분 테이블)
do $$
begin
    if not exists (select 1 from pg_roles where rolname = 'gongzzang_app_writer') then
        create role gongzzang_app_writer;
    end if;
end $$;
grant connect on database gongzzang to gongzzang_app_writer;
grant usage on schema public to gongzzang_app_writer;
grant select, insert, update, delete on all tables in schema public to gongzzang_app_writer;

-- audit_log는 INSERT만, UPDATE/DELETE 박탈 (immutable)
revoke update, delete on audit_log from gongzzang_app_writer;

-- 2. 읽기 전용 (분석·리포트)
do $$
begin
    if not exists (select 1 from pg_roles where rolname = 'gongzzang_app_reader') then
        create role gongzzang_app_reader;
    end if;
end $$;
grant connect on database gongzzang to gongzzang_app_reader;
grant usage on schema public to gongzzang_app_reader;
grant select on all tables in schema public to gongzzang_app_reader;

-- 3. audit archiver (audit_log SELECT + DELETE만, archive worker 전용)
do $$
begin
    if not exists (select 1 from pg_roles where rolname = 'gongzzang_audit_archiver') then
        create role gongzzang_audit_archiver;
    end if;
end $$;
grant connect on database gongzzang to gongzzang_audit_archiver;
grant usage on schema public to gongzzang_audit_archiver;
grant select, delete on audit_log to gongzzang_audit_archiver;

-- 실제 사용자 (Pulumi에서 생성)
-- gongzzang_api_user (gongzzang_app_writer 역할 부여) — services/api 가 사용
-- gongzzang_analytics_user (gongzzang_app_reader) — 분석 도구
-- gongzzang_archiver_user (gongzzang_audit_archiver) — services/worker archive job
