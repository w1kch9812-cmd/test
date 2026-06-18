# traffic-auth-policy-registry/

Traffic/auth policy source fragments.

Authoritative edits happen in this directory. The compatibility aggregate
`../traffic-auth-policy-registry.v1.json` is generated from these fragments by:

```powershell
./scripts/ci/generate-traffic-auth-policy-registry.ps1 -Root .
```

`./scripts/ci/generate-traffic-auth-policy.ps1 -Root .` runs the registry
generator first, then regenerates the downstream TypeScript, Rust, and edge
policy projections.

The registry checker compares the aggregate against these fragments, so manual
aggregate edits drift from source and fail CI.
