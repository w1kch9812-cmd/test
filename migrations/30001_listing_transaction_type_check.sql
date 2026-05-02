-- V003_01: listing transaction_type × deposit/monthly_rent cross-field CHECK
-- spec § 5.1 누락 invariant 보강 (sub-project 2a-fixup)
--
-- 거래 유형별 금액 필드 invariant:
--   sale          → deposit NULL,    monthly_rent NULL
--   monthly_rent  → deposit NOT NULL, monthly_rent NOT NULL
--   jeonse        → deposit NOT NULL, monthly_rent NULL

alter table listing
    add constraint listing_transaction_fields_chk
    check (
        (transaction_type = 'sale'
         and deposit_krw is null
         and monthly_rent_krw is null)
        or
        (transaction_type = 'monthly_rent'
         and deposit_krw is not null
         and monthly_rent_krw is not null)
        or
        (transaction_type = 'jeonse'
         and deposit_krw is not null
         and monthly_rent_krw is null)
    );
