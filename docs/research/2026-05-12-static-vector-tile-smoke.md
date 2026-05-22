# Static Vector Tile Smoke Measurement - 2026-05-12

## Scope

This note records local smoke evidence for ADR 0036. It is **not** a nationwide production cost estimate.

Measured path:

```text
var/gold/v1/parcels
```

The directory contains a small local flat `.pbf` sample generated before this ADR. It is useful for checking the shape of the runtime artifact:

```text
<version>/<layer>/{z}/{x}/{y}.pbf
```

## Command

```powershell
$root = Resolve-Path 'var/gold/v1/parcels'
$files = Get-ChildItem -Path $root -Recurse -Filter '*.pbf' -File
$byZoom = $files |
  ForEach-Object {
    $rel = $_.FullName.Substring($root.Path.Length + 1)
    $zoom = $rel.Split([IO.Path]::DirectorySeparatorChar)[0]
    [pscustomobject]@{ Zoom=$zoom; Length=$_.Length }
  } |
  Group-Object Zoom |
  Sort-Object {[int]$_.Name}
$total = ($files | Measure-Object -Property Length -Sum).Sum
```

## Result

| Zoom | Tile count | Total bytes | Average bytes | Max bytes |
|---:|---:|---:|---:|---:|
| 14 | 15 | 29,629 | 1,975.27 | 12,913 |
| 15 | 35 | 38,680 | 1,105.14 | 13,784 |
| 16 | 61 | 47,748 | 782.75 | 8,400 |
| 17 | 124 | 65,246 | 526.18 | 4,518 |
| **Total** | **235** | **181,303** | - | - |

## Interpretation

- The current local artifact uses the ADR 0021 flat tile shape.
- The smoke sample is small enough to verify cheaply in local/dev flows.
- Production cost cannot be inferred from this sample. A nationwide measurement must run against the real production build output and record object count, total bytes, zoom distribution, and R2 egress assumptions.

## ADR 0036 Follow-up Inputs

The first production-grade measurement must record:

- layer name
- active version
- PMTiles bytes
- PMTiles sha256
- flat tile count
- flat tile total bytes
- zoom distribution
- average tile bytes
- max tile bytes
- configured landmark coverage result
- estimated R2 object operation and egress cost
