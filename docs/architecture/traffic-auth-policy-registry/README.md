# traffic-auth-policy-registry/

Traffic/auth policy source fragments.

The hand-edited SSOT is the aggregate `../traffic-auth-policy-registry.v1.json`.
These fragments document the policy in reviewable, folder-shaped pieces; keep them
in sync with the aggregate when you change a policy.

After editing the registry, regenerate the downstream TypeScript, Rust, and edge
policy projections with the Rust generator:

```sh
cargo run -p api --bin generate-traffic-auth-policy
```

The generator reads `../traffic-auth-policy-registry.v1.json` and rewrites the six
committed policy artifacts (two `.ts`, two `.rs`, two `.json`), so the generated
files always reproduce byte-for-byte from the registry.
