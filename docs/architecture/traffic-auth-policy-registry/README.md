# traffic-auth-policy-registry/

> 이 폴더의 split fragment JSON 들은 제거되었습니다. 단일 SSOT 는 상위
> 폴더의 aggregate `../traffic-auth-policy-registry.v1.json` 하나입니다.

Traffic/auth 정책의 **유일한 출처(SSOT)** 는 손으로 직접 편집하는 aggregate
`../traffic-auth-policy-registry.v1.json` 입니다. 예전에는 같은 정책을 작은
`00-*.json` ~ `80-*.json` fragment 로도 복제해 두고 손으로 sync 했지만, 그
fragment 들은 어떤 코드도 읽지 않는 죽은 사본이었으므로 삭제했습니다. 정책은
aggregate 한 곳에서만 수정합니다.

정책을 수정한 뒤에는 Rust 생성기로 다운스트림 TypeScript / Rust / edge 정책
산출물을 재생성하세요:

```sh
cargo run -p api --bin generate-traffic-auth-policy
```

생성기는 `../traffic-auth-policy-registry.v1.json` 만 읽어 6개의 커밋된 산출물
(`.ts` 2개, `.rs` 2개, `.json` 2개) 을 다시 씁니다. 따라서 산출물은 항상
aggregate 로부터 byte-for-byte 로 재현됩니다. CI 의 traffic-auth drift 가드가
생성기 실행 후 `git diff --exit-code` 로 이 재현성을 강제합니다.
