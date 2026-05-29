# crates/domain/shared-kernel

Legacy location for shared kernel notes. The active crate is
`crates/domain/core/shared-kernel`.

## Value Objects

- `Pnu`: 19-digit parcel identifier
- `BusinessNumber`: 10-digit Korean business number, displayed as `000-00-00000`
- `BrokerLicense`: licensed broker identifier
- `Money`: KRW amount
- `Area`: positive area value
- `Geometry`: PostGIS-compatible geometry with explicit SRID
- `Email`, `PhoneKr`: validated contact values
- `Ulid`: domain-prefixed identifier
- `Timestamp`: UTC storage with KST presentation conversion

## Rules

- Dependencies stay minimal.
- Invalid values are rejected at construction.
- Domain crates depend inward on shared-kernel; shared-kernel does not depend outward.
