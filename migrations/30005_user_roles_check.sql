-- V003_05: user.roles 원소가 7 종 enum 값 중 하나임을 보장
-- spec § 8.2, sub-project 3 (Auth)
--
-- UserRole enum (crates/domain/core/user/src/entity.rs:37-52):
--   Buyer / Seller / Broker / Developer / Enterprise / Operator / Admin

alter table "user"
    add constraint user_roles_valid_chk check (
        roles <@ array['Buyer','Seller','Broker','Developer','Enterprise','Operator','Admin']::text[]
    );
