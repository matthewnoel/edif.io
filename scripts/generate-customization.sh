#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/data/player-customization.json"
RS_OUT="$REPO_ROOT/core/src/customization.rs"
TS_OUT="$REPO_ROOT/client/src/lib/customization.ts"

CHECK_MODE=false
if [[ "${1:-}" == "--check" ]]; then
  CHECK_MODE=true
fi

if [[ ! -f "$SOURCE" ]]; then
  echo "Source file missing: $SOURCE" >&2
  exit 1
fi

# --- Per-language array body emitters ------------------------------------
# Each prints the bracket-interior of the array — items only, no surrounding
# `[` or `];`. Output for the last item omits a trailing comma (TS) or keeps
# one (Rust idiom).

emit_rust_items() {
  local key="$1"
  jq -r --arg k "$key" '.[$k] | map("    \"" + . + "\",") | join("\n")' "$SOURCE"
}

emit_ts_items() {
  # Single-quote each item; join with ",\n" so the last item has no trailing
  # comma (matches prettier's `trailingComma: "none"`).
  local key="$1"
  jq -r --arg k "$key" '.[$k] | map("\t'\''" + . + "'\''") | join(",\n")' "$SOURCE"
}

build_rust() {
  local adjectives nouns palette
  adjectives="$(emit_rust_items adjectives)"
  nouns="$(emit_rust_items nouns)"
  palette="$(emit_rust_items palette)"

  cat <<EOF
// !!! GENERATED FILE — DO NOT EDIT !!!
// Source: data/player-customization.json
// To regenerate: run \`make generate\`.

#[rustfmt::skip]
pub static ADJECTIVES: &[&str] = &[
${adjectives}
];

#[rustfmt::skip]
pub static NOUNS: &[&str] = &[
${nouns}
];

#[rustfmt::skip]
pub static PALETTE: &[&str] = &[
${palette}
];
EOF
}

build_ts() {
  local adjectives nouns palette
  adjectives="$(emit_ts_items adjectives)"
  nouns="$(emit_ts_items nouns)"
  palette="$(emit_ts_items palette)"

  cat <<EOF
// !!! GENERATED FILE — DO NOT EDIT !!!
// Source: data/player-customization.json
// To regenerate: run \`make generate\`.

export const ADJECTIVES = [
${adjectives}
];

export const NOUNS = [
${nouns}
];

export const PALETTE = [
${palette}
];
EOF
}

# --- Apply ----------------------------------------------------------------

apply() {
  local content="$1"
  local target="$2"

  if $CHECK_MODE; then
    if ! diff -u "$target" <(printf '%s\n' "$content") >&2; then
      echo "DRIFT: $target is out of date. Run: make generate" >&2
      return 1
    fi
  else
    printf '%s\n' "$content" > "$target"
  fi
}

drift=0
apply "$(build_rust)" "$RS_OUT" || drift=1
apply "$(build_ts)" "$TS_OUT" || drift=1

if $CHECK_MODE; then
  if [[ $drift -ne 0 ]]; then
    echo "Generated customization files are out of date." >&2
    exit 1
  fi
  echo "Generated customization files are up to date."
else
  echo "Generated customization files written."
fi
