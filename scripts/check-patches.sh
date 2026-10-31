#!/usr/bin/env bash
# Verify that patches/series is complete and that every patch applies cleanly
# to every supported upstream tag.
#
#   ./scripts/check-patches.sh                 # every tag >= MIN_VERSION
#   ./scripts/check-patches.sh epoch-1.11.0    # selected tag(s)
#   ./scripts/check-patches.sh --fmt           # also run cargo fmt --check
#
# Tags are fetched from upstream when possible; local tags are used as a
# fallback so the script still works offline.
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

fmt=0
if [[ ${1:-} == --fmt ]]; then
  fmt=1
  shift
fi

upstream_url=${UPSTREAM_URL:-https://github.com/pop-os/cosmic-comp.git}
if ! git fetch --quiet --tags "$upstream_url" 'refs/tags/epoch-*:refs/tags/epoch-*'; then
  echo "warning: could not fetch tags from $upstream_url; using local tags" >&2
fi

series=patches/series
if [[ ! -f $series ]]; then
  echo "missing $series" >&2
  exit 1
fi

# The series is the explicit order. Fail if it drifts from patches/common/.
declare -A listed=()
while IFS= read -r line || [[ -n $line ]]; do
  line=${line%%#*}
  line=${line//[[:space:]]/}
  [[ -z $line ]] && continue
  if [[ -n ${listed[$line]:-} ]]; then
    echo "duplicate entry in $series: $line" >&2
    exit 1
  fi
  if [[ ! -f patches/common/$line ]]; then
    echo "$series lists a missing patch: $line" >&2
    exit 1
  fi
  listed[$line]=1
done < "$series"

for patch in patches/common/*.patch; do
  name=$(basename "$patch")
  if [[ -z ${listed[$name]:-} ]]; then
    echo "$patch is not listed in $series" >&2
    exit 1
  fi
done
echo "series: ${#listed[@]} common patch(es) listed"

min_version=${MIN_VERSION:-}
if [[ -z $min_version ]]; then
  min_version=$(sed -n "s/^[[:space:]]*MIN_VERSION:[[:space:]]*'\([^']*\)'.*/\1/p" \
    .github/workflows/release.yml | head -n 1)
  min_version=${min_version:-1.10.0}
fi

tags=("$@")
if (( ${#tags[@]} == 0 )); then
  while IFS= read -r tag; do
    version=${tag#epoch-}
    [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
    if [[ $(printf '%s\n%s\n' "$min_version" "$version" | sort -V | head -n 1) == "$min_version" ]]; then
      tags+=("$tag")
    fi
  done < <(git tag --list 'epoch-*' | sort -V)
fi

if (( ${#tags[@]} == 0 )); then
  echo "no supported tags found (MIN_VERSION=$min_version)" >&2
  exit 1
fi

# Normalize "1.11.0" and "epoch-1.11.0" to the same tag name.
normalized=()
for tag in "${tags[@]}"; do
  tag="epoch-${tag#epoch-}"
  if ! git rev-parse --verify --quiet "refs/tags/$tag^{commit}" >/dev/null; then
    echo "unknown tag: $tag" >&2
    exit 1
  fi
  normalized+=("$tag")
done
tags=("${normalized[@]}")

echo "checking ${#tags[@]} tag(s): ${tags[*]}"

work=$(mktemp -d)

cleanup() {
  local w
  for w in "$work"/*; do
    if [[ -d $w ]]; then
      git worktree remove --force "$w" >/dev/null 2>&1 || true
    fi
  done
  rm -rf "$work"
}
trap cleanup EXIT

apply_patch() {
  local patch=$1
  if ! git apply --whitespace=nowarn -p1 "$patch"; then
    echo "FAILED: $patch" >&2
    return 1
  fi
}

for tag in "${tags[@]}"; do
  version=${tag#epoch-}
  tree="$work/$tag"
  echo "==> $tag"
  git worktree add --detach --quiet "$tree" "$tag"

  (
    cd "$tree"
    while IFS= read -r name || [[ -n $name ]]; do
      name=${name%%#*}
      name=${name//[[:space:]]/}
      [[ -z $name ]] && continue
      apply_patch "$repo_dir/patches/common/$name"
    done < "$repo_dir/$series"

    if [[ -d "$repo_dir/patches/$version" ]]; then
      for patch in "$repo_dir/patches/$version"/*.patch; do
        [[ -e $patch ]] || continue
        apply_patch "$patch"
      done
    fi

    if (( fmt )); then
      cargo fmt --all -- --check
    fi
  )

  git worktree remove --force "$tree"
done

echo "all patches apply cleanly"
