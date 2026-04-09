#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHECK_MODE=false

if [[ "${1:-}" == "--check" ]]; then
  CHECK_MODE=true
fi

# --- Adapter discovery ---

declare -a ADAPTER_NAMES=()
declare -a ADAPTER_PKGS=()
declare -a ADAPTER_DESCS=()

for cargo_toml in "$REPO_ROOT"/adapters/*/Cargo.toml; do
  dir="$(dirname "$cargo_toml")"
  name="$(basename "$dir")"

  pkg="$(awk -F'"' '/^name *=/{print $2; exit}' "$cargo_toml")"
  if [[ -z "$pkg" ]]; then
    pkg="$name"
  fi

  desc=""
  agents_md="$dir/AGENTS.md"
  if [[ -f "$agents_md" ]]; then
    desc="$(awk '/^## Scope$/{found=1; next} found && /^- /{sub(/^- */, ""); print; exit}' "$agents_md")"
  fi
  if [[ -z "$desc" ]]; then
    desc="$name adapter."
  fi

  ADAPTER_NAMES+=("$name")
  ADAPTER_PKGS+=("$pkg")
  ADAPTER_DESCS+=("$desc")
done

# --- Marker replacement ---
# Replaces content between open/close marker comments with content from a file.

replace_markers() {
  local file="$1"
  local marker="$2"
  local content_file="$3"
  local open="<!-- GENERATED:${marker} -->"
  local close="<!-- /GENERATED:${marker} -->"
  local inside=0

  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "$open" ]]; then
      printf '%s\n' "$line"
      cat "$content_file"
      inside=1
    elif [[ "$line" == "$close" ]]; then
      inside=0
      printf '%s\n' "$line"
    elif [[ $inside -eq 0 ]]; then
      printf '%s\n' "$line"
    fi
  done < "$file"
}

# --- Build generated content for each section ---

build_repo_adapter_list() {
  for i in "${!ADAPTER_NAMES[@]}"; do
    printf '  - `adapters/%s`: %s\n' "${ADAPTER_NAMES[$i]}" "${ADAPTER_DESCS[$i]}"
  done
}

build_registered_adapters() {
  for i in "${!ADAPTER_NAMES[@]}"; do
    printf -- '- `%s` (`%s`): %s\n' "${ADAPTER_NAMES[$i]}" "${ADAPTER_PKGS[$i]}" "${ADAPTER_DESCS[$i]}"
  done
}

# --- Apply to a file (in-place or check) ---

process_file() {
  local file="$1"
  shift

  [[ -f "$file" ]] || return 0

  local tmpdir
  tmpdir="$(mktemp -d)"
  trap "rm -rf '$tmpdir'" RETURN

  cp "$file" "$tmpdir/current"

  while [[ $# -ge 2 ]]; do
    local marker="$1"
    local content="$2"
    shift 2

    printf '%s\n' "$content" > "$tmpdir/content"
    replace_markers "$tmpdir/current" "$marker" "$tmpdir/content" > "$tmpdir/next"
    mv "$tmpdir/next" "$tmpdir/current"
  done

  if $CHECK_MODE; then
    if ! diff -u "$file" "$tmpdir/current" >&2; then
      echo "DRIFT: $file is out of date. Run: bash scripts/generate-agents-md.sh" >&2
      return 1
    fi
  else
    cp "$tmpdir/current" "$file"
  fi
}

# --- Main ---

repo_adapter_list="$(build_repo_adapter_list)"
registered_adapters="$(build_registered_adapters)"

drift=0

process_file "$REPO_ROOT/AGENTS.md" \
  "REPO_ADAPTER_LIST" "$repo_adapter_list" \
  || drift=1

process_file "$REPO_ROOT/server/AGENTS.md" \
  "REGISTERED_ADAPTERS" "$registered_adapters" \
  || drift=1

if $CHECK_MODE; then
  if [[ $drift -ne 0 ]]; then
    echo "AGENTS.md files are out of date." >&2
    exit 1
  fi
  echo "AGENTS.md files are up to date."
else
  echo "AGENTS.md files regenerated."
fi
