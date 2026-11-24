#!/usr/bin/env bash
# Verify that patches/series is complete and that every patch applies cleanly
# to every supported upstream tag.
#
#   ./scripts/check-patches.sh                  # every tag >= MIN_VERSION
#   ./scripts/check-patches.sh epoch-1.11.0     # selected tag(s)
#   ./scripts/check-patches.sh --fmt            # also run cargo fmt --check
#   ./scripts/check-patches.sh --ref HEAD       # also check an arbitrary ref
#   ./scripts/check-patches.sh --offline        # never fetch; local tags only
#   ./scripts/check-patches.sh --include-rc     # also check epoch-X.Y.Z-rc.N tags
#
# Upstream epoch tags are fetched when possible, with local tags as a fallback.
# --offline makes "use local tags only" explicit (and avoids a failing fetch).
# Fork rc tags such as epoch-1.12.0-rc.1 are skipped by default; --include-rc
# opts in to checking them too.
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

fmt=0
offline=0
include_rc=0
selected=()
extra_refs=()
args=("$@")
while (( ${#args[@]} > 0 )); do
  arg=${args[0]}
  args=("${args[@]:1}")
  case $arg in
    --fmt) fmt=1 ;;
    --offline) offline=1 ;;
    --include-rc) include_rc=1 ;;
    --ref)
      if (( ${#args[@]} == 0 )); then
        echo "--ref needs a value" >&2
        exit 1
      fi
      extra_refs+=("${args[0]}")
      args=("${args[@]:1}")
      ;;
    *) selected+=("$arg") ;;
  esac
done

upstream_url=${UPSTREAM_URL:-https://github.com/pop-os/cosmic-comp.git}
if (( offline )); then
  echo "offline mode: using local tags only"
elif ! git fetch --quiet --tags "$upstream_url" 'refs/tags/epoch-*:refs/tags/epoch-*'; then
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
  min_version=${min_version:-1.11.0}
fi

tags=("${selected[@]}")
if (( ${#tags[@]} == 0 )); then
  while IFS= read -r tag; do
    version=${tag#epoch-}
    if (( include_rc )); then
      [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$ ]] || continue
    else
      # Explicitly skip fork rc tags (e.g. epoch-1.12.0-rc.1).
      [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || continue
    fi
    base_version=${version%%-rc*}
    if [[ $(printf '%s\n%s\n' "$min_version" "$base_version" | sort -V | head -n 1) == "$min_version" ]]; then
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

check_ref() {
  local label=$1 rev=$2 version=$3
  local tree="$work/$label"
  echo "==> $label"
  git worktree add --detach --quiet "$tree" "$rev"

  (
    cd "$tree"
    # A version-specific patch with the same name replaces the common one.
    declare -A applied_names=()
    while IFS= read -r name || [[ -n $name ]]; do
      name=${name%%#*}
      name=${name//[[:space:]]/}
      [[ -z $name ]] && continue
      patch="$repo_dir/patches/common/$name"
      if [[ -n $version && -f "$repo_dir/patches/$version/$name" ]]; then
        patch="$repo_dir/patches/$version/$name"
      fi
      apply_patch "$patch"
      applied_names[$name]=1
    done < "$repo_dir/$series"

    if [[ -n $version && -d "$repo_dir/patches/$version" ]]; then
      for patch in "$repo_dir/patches/$version"/*.patch; do
        [[ -e $patch ]] || continue
        name=$(basename "$patch")
        [[ -n ${applied_names[$name]:-} ]] && continue
        apply_patch "$patch"
      done
    fi

    if (( fmt )); then
      # The upstream tag pins its own toolchain (e.g. 1.93), which is not the
      # one `rustup component add rustfmt` at the repo root installed for.
      # Install rustfmt for whichever toolchain this worktree resolves to.
      rustup component add rustfmt
      cargo fmt --all -- --check
    fi
  )

  git worktree remove --force "$tree"
}

for tag in "${tags[@]}"; do
  check_ref "$tag" "$tag" "${tag#epoch-}"
done
for ref in "${extra_refs[@]}"; do
  safe=${ref//\//_}
  safe=${safe//:/_}
  check_ref "$safe" "$ref" ""
done

echo "all patches apply cleanly"
