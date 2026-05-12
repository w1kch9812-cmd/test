#!/usr/bin/env bash
# Catalog M1 boundary guardrail — ADR 0030 / 0031 / 0034.
#
# During M1 phase, gongzzang3 is sole owner of catalog (industrial-complex /
# parcel / building / manufacturer). After M3.2 cutover, ownership moves to
# platform-core. Any new mutation surface added now will need rewrite — so
# this hook blocks it at commit time.
#
# Checks:
# 1. catalog domain crates must remain reader-only (no repository / writer /
#    save / sqlx-query surface)
# 2. no direct SQL writes (INSERT / UPDATE / DELETE) to catalog tables anywhere
#    in the workspace (services/, crates/, apps/)
# 3. no HTTP mutation routes (POST / PUT / PATCH / DELETE) targeting catalog
#    resources anywhere a router is registered (services/, crates/, apps/)
#
# Codex stop-time review fix: route check moved from single-line grep
# (bypassable via multi-line `.route(...)` blocks, qualified
# `axum::routing::post`, or `.post(handler)` chained on a `.route()` call) to a
# perl multi-line scanner that paren-balances the route call.

set -euo pipefail

catalog_domain_dirs=(
  "crates/domain/core/industrial-complex"
  "crates/domain/core/parcel"
  "crates/domain/core/building"
  "crates/domain/core/manufacturer"
)

# Scan roots for SQL-write + HTTP-route checks. Adding a new router crate
# automatically increases coverage as long as it lives under one of these.
scan_roots=("services" "crates" "apps")

fail=0

report() {
  echo "catalog-m1-boundary: $1" >&2
  fail=1
}

# ── 1) catalog domain crates — reader-only ──────────────────────────────────
for dir in "${catalog_domain_dirs[@]}"; do
  [ -d "$dir/src" ] || continue

  if find "$dir/src" -type f \
       \( -name "repository.rs" -o -name "*repository*.rs" \
        -o -name "writer.rs" -o -name "*writer*.rs" \) \
       | grep -q .; then
    report "$dir must remain reader-only during M1; repository/writer modules belong to platform-core after cutover."
  fi

  if grep -RInEi \
       'pub[[:space:]]+mod[[:space:]]+(repository|writer)|trait[[:space:]]+[A-Za-z0-9_]*(Repository|Writer)\b|fn[[:space:]]+(save|insert|update|delete|upsert|create|persist|store|put|patch|replace|sync|write)\b|sqlx::query|[[:space:]](insert[[:space:]]+into|update|delete[[:space:]]+from)[[:space:]]' \
       "$dir/src" >/dev/null; then
    report "$dir contains write/repository surface. Catalog M1 allows entity/value/read ports only."
  fi
done

# ── 2) direct SQL writes to catalog owner tables ────────────────────────────
# Multi-line SQL (e.g. sqlx::query!("UPDATE\n  parcel SET ...")) is caught by
# perl -0777 slurping each .rs file.
export CATALOG_TABLES_RE='(industrial_complex|industrial_complexes|parcel|parcels|building|buildings|manufacturer|manufacturers)'

for root in "${scan_roots[@]}"; do
  [ -d "$root" ] || continue
  while IFS= read -r -d '' file; do
    hits=$(perl -0777 -ne '
      my $tables = $ENV{CATALOG_TABLES_RE};
      while (m{\b(INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+(?:"?\w+"?\.)?"?($tables)"?\b}gis) {
        print "  $1 $2\n";
      }
    ' "$file")
    if [ -n "$hits" ]; then
      report "$file: catalog SQL write detected (M1 read-only):"
      echo "$hits" >&2
    fi
  done < <(find "$root" -type f -name "*.rs" -print0)
done

# ── 3) HTTP mutation routes targeting catalog resources ─────────────────────
# Multi-line `.route(...)` blocks + chained `.post(...)` / `.put(...)` etc. on
# router builders are caught via perl paren-balanced extraction.
export CATALOG_PATH_RE='(parcels?|buildings?|industrial[-_]complexes?|manufacturers?)'

for root in "${scan_roots[@]}"; do
  [ -d "$root" ] || continue
  while IFS= read -r -d '' file; do
    violations=$(perl -0777 -ne '
      my $path_re = $ENV{CATALOG_PATH_RE};
      # Extract every .route("...path...", <handler-expr>) block. Handler-expr
      # may span multiple lines. We paren-balance from after the path string
      # to find the closing `)` of the route() call, regardless of newlines.
      while (m{\.route\s*\(\s*"([^"]*)"\s*,}sg) {
        my $path = $1;
        # Save @+ from outer match BEFORE running inner regex (inner match
        # clobbers @-/@+ globals — Codex stop-time review finding).
        my $tail_start = $+[0];
        my $resume_pos = pos();
        next unless $path =~ m{/$path_re(/|$|\?)};
        # paren-balance from $tail_start to find closing `)` of route call.
        my $pos = $tail_start;
        my $depth = 1;
        while ($pos < length($_)) {
          my $ch = substr($_, $pos, 1);
          if ($ch eq "(") { $depth++; }
          elsif ($ch eq ")") {
            $depth--;
            last if $depth == 0;
          }
          $pos++;
        }
        # restore pos() for outer while loop (inner regex may have moved it).
        pos($_) = $resume_pos;
        next if $depth != 0;
        my $block = substr($_, $tail_start, $pos - $tail_start);
        if ($block =~ /\b(?:post|put|patch|delete)\s*\(/
            || $block =~ /\baxum::routing::(?:post|put|patch|delete)\b/
            || $block =~ /\bMethod::(?:POST|PUT|PATCH|DELETE)\b/) {
          print "  $path -> mutation method in route handler\n";
        }
      }
      # Chained `.get(h).post(h2)` inside the route block is already caught
      # by the paren-balanced extraction above. No separate scan needed.
    ' "$file")
    if [ -n "$violations" ]; then
      report "$file: catalog mutation route detected (M1 read-only):"
      echo "$violations" >&2
    fi
  done < <(find "$root" -type f -name "*.rs" -print0)
done

if [ "$fail" -ne 0 ]; then
  cat >&2 <<'EOF'

ADR references:
- docs/adr/0030-three-service-architecture.md
- docs/adr/0031-platform-core-bounded-contexts.md
- docs/adr/0034-catalog-ownership-handover-to-platform-core.md

EOF
  exit 1
fi
