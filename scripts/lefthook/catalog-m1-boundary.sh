#!/usr/bin/env bash
# Catalog M1 boundary guardrail — ADR 0030 / 0031 / 0034.
#
# During M1 phase, gongzzang3 is sole owner of catalog (industrial-complex /
# parcel / building / manufacturer). After M3.2 cutover, ownership moves to
# platform-core. Any new mutation surface added now will need rewrite — so
# this hook blocks it at commit time.
#
# Strategy: whitelist instead of blacklist. Catalog HTTP routes must only use
# `get` / `head` handlers. Anything else (post / put / patch / delete / on /
# any / MethodRouter::new().post / Method::POST / .nest) is flagged.
#
# Checks:
# 1. catalog domain crates must remain reader-only (no repository / writer /
#    save / sqlx-query surface)
# 2. no direct SQL writes (INSERT / UPDATE / DELETE) to catalog tables anywhere
#    in the workspace (services/, crates/, apps/)
# 3. catalog HTTP routes are read-only (whitelist: get/head only)
# 4. catalog path constants are explicitly surfaced for review

set -euo pipefail

catalog_domain_dirs=(
  "crates/domain/core/industrial-complex"
  "crates/domain/core/parcel"
  "crates/domain/core/building"
  "crates/domain/core/manufacturer"
)

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

# ── 3) HTTP routes / nest blocks against catalog paths ──────────────────────
export CATALOG_PATH_RE='(parcels?|buildings?|industrial[-_]complexes?|manufacturers?)'

for root in "${scan_roots[@]}"; do
  [ -d "$root" ] || continue
  while IFS= read -r -d '' file; do
    violations=$(perl -0777 -ne '
      my $path_re = $ENV{CATALOG_PATH_RE};

      # ── 3a) `.route("path", handler)` whitelist ─────────────────────────
      while (m{\.route\s*\(\s*"([^"]*)"\s*,}sg) {
        my $path = $1;
        # Save outer match offsets BEFORE inner regex (inner clobbers @-/@+
        # globals — Codex stop-time review finding).
        my $tail_start = $+[0];
        my $resume_pos = pos();
        next unless $path =~ m{/$path_re(/|$|\?)};
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
        pos($_) = $resume_pos;
        next if $depth != 0;
        my $block = substr($_, $tail_start, $pos - $tail_start);

        # Whitelist: every method-router call inside the block must be get()
        # or head(). post / put / patch / delete / on / any / etc. = mutation.
        my @plain = ($block =~ /(?:^|[^\w:])(\w+)\s*\(/g);
        my @qualified = ($block =~ /\baxum::routing::(\w+)\b/g);
        my %seen;
        for my $m (@plain, @qualified) {
          next if $seen{$m}++;
          next unless $m =~ /^(get|head|post|put|patch|delete|options|trace|connect|any|on|on_service|on_method|method_router|fallback)$/i;
          if ($m !~ /^(get|head)$/i) {
            print "  $path -> handler uses \"$m\" (M1 allows only get/head)\n";
            last;
          }
        }
        # `MethodFilter::POST` / `Method::POST` constants used with `on()`.
        if ($block =~ /\bMethod(?:Filter)?::(POST|PUT|PATCH|DELETE|TRACE|CONNECT|OPTIONS)\b/i) {
          print "  $path -> Method::$1 constant in handler (M1 read-only)\n";
        }
      }

      # ── 3b) `.nest("path", sub)` — sub-router may live elsewhere ─────────
      while (m{\.nest(?:_service)?\s*\(\s*"([^"]*)"\s*,}sg) {
        my $path = $1;
        my $resume_pos = pos();
        if ($path =~ m{/$path_re(/|$|\?)}) {
          print "  $path -> .nest(\"$path\", ...) sub-router (M1 forbids catalog mutation surface)\n";
        }
        pos($_) = $resume_pos;
      }
    ' "$file")
    if [ -n "$violations" ]; then
      report "$file: catalog mutation route detected (M1 read-only):"
      echo "$violations" >&2
    fi
  done < <(find "$root" -type f -name "*.rs" -print0)
done

# ── 4) Path constants pointing at catalog resources ─────────────────────────
# Resolve catalog-path consts to their identifier names, then re-scan for
# `.route(IDENT, ...)` blocks and apply the same whitelist as step 3.
for root in "${scan_roots[@]}"; do
  [ -d "$root" ] || continue
  while IFS= read -r -d '' file; do
    violations=$(perl -0777 -ne '
      my $path_re = $ENV{CATALOG_PATH_RE};

      # Collect catalog-path const/static identifiers in this file.
      my @catalog_idents;
      while (m{\b(?:const|static)\s+(\w+)\s*:\s*&?(?:'"'"'static\s+)?str\s*=\s*"([^"]*)"}sg) {
        my ($name, $val) = ($1, $2);
        next unless $val =~ m{/$path_re(/|$|\?)};
        push @catalog_idents, [$name, $val];
      }
      # For each const, check `.route(IDENT, <handler>)` and `.nest(IDENT, ...)`.
      for my $pair (@catalog_idents) {
        my ($name, $val) = @$pair;
        # route(IDENT, ...) — paren-balance the handler.
        while (m{\.route\s*\(\s*\Q$name\E\s*,}sg) {
          my $tail_start = $+[0];
          my $resume_pos = pos();
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
          pos($_) = $resume_pos;
          next if $depth != 0;
          my $block = substr($_, $tail_start, $pos - $tail_start);
          my @plain = ($block =~ /(?:^|[^\w:])(\w+)\s*\(/g);
          my @qualified = ($block =~ /\baxum::routing::(\w+)\b/g);
          my %seen;
          for my $m (@plain, @qualified) {
            next if $seen{$m}++;
            next unless $m =~ /^(get|head|post|put|patch|delete|options|trace|connect|any|on|on_service|on_method|method_router|fallback)$/i;
            if ($m !~ /^(get|head)$/i) {
              print "  $name (=\"$val\") .route -> handler uses \"$m\" (M1 allows only get/head)\n";
              last;
            }
          }
          if ($block =~ /\bMethod(?:Filter)?::(POST|PUT|PATCH|DELETE|TRACE|CONNECT|OPTIONS)\b/i) {
            print "  $name (=\"$val\") .route -> Method::$1 constant (M1 read-only)\n";
          }
        }
        # nest(IDENT, ...) — sub-router opacity = forbid.
        while (m{\.nest(?:_service)?\s*\(\s*\Q$name\E\s*,}sg) {
          print "  $name (=\"$val\") .nest sub-router (M1 forbids catalog mutation surface)\n";
        }
      }
    ' "$file")
    if [ -n "$violations" ]; then
      report "$file: catalog path constant used in mutation context (M1 read-only):"
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
