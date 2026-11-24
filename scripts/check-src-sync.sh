#!/usr/bin/env bash
# Verify that the in-tree `src/` snapshot is exactly upstream base + patches/series.
#
# The PKGBUILD/RPM builds apply the series to a fresh upstream checkout, while
# the Nix flake and plain `cargo build` use the committed `src/` tree. If the
# two drift apart, those build paths silently produce different compositors.
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

base=${SRC_SYNC_BASE:-0681eaf6}
if ! git rev-parse --verify --quiet "${base}^{commit}" >/dev/null; then
  echo "unknown source base: $base" >&2
  exit 1
fi

work=$(mktemp -d)
trap 'git worktree remove --force "$work" >/dev/null 2>&1 || true; rm -rf "$work"' EXIT

git worktree add --detach --quiet "$work" "$base"
(
  cd "$work"
  while IFS= read -r name || [[ -n $name ]]; do
    name=${name%%#*}
    name=${name//[[:space:]]/}
    [[ -z $name ]] && continue
    git apply --whitespace=nowarn -p1 "$repo_dir/patches/common/$name"
  done < "$repo_dir/patches/series"
)

status=0
if ! diff -qr "$work/src" "$repo_dir/src"; then
  echo "ERROR: src/ is out of sync with patches/series" >&2
  status=1
fi

# These directories are not touched by any patch and must match the base.
for dir in cosmic-comp-config cosmic-comp-ipc data resources; do
  if ! diff -qr "$work/$dir" "$repo_dir/$dir"; then
    echo "ERROR: $dir is out of sync with upstream $base" >&2
    status=1
  fi
done

if (( status != 0 )); then
  echo "Regenerate the tree from upstream $base with the patch series applied." >&2
  exit 1
fi

echo "tree is in sync with patches/series"
