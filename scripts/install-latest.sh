#!/usr/bin/env bash
# Download and install the newest cosmic-comp-traffic package from GitHub releases.
#
#   ./scripts/install-latest.sh            # latest release
#   ./scripts/install-latest.sh epoch-1.11.0
set -euo pipefail

repo=${REPO:-grigio/cosmic-comp-traffic}
tag=${1:-}

command -v gh >/dev/null || { echo "gh (github-cli) is required" >&2; exit 1; }
command -v pacman >/dev/null || { echo "pacman is required" >&2; exit 1; }

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

if [[ -n $tag ]]; then
  gh release download "$tag" --repo "$repo" --pattern '*.pkg.tar.zst*' --dir "$dir"
else
  gh release download --repo "$repo" --pattern '*.pkg.tar.zst*' --dir "$dir"
fi

(cd "$dir" && sha256sum --check --quiet ./*.pkg.tar.zst.sha256)

# install the compositor package, not the optional -debug one
pkgs=()
for f in "$dir"/*.pkg.tar.zst; do
  [[ $(basename "$f") == *-debug-* ]] && continue
  pkgs+=("$f")
done
if [[ ${#pkgs[@]} -eq 0 ]]; then
  echo "no package found in the release" >&2
  exit 1
fi
# a rebuilt release can still carry an older pkgrel: install the newest one
mapfile -t pkgs < <(printf '%s\n' "${pkgs[@]}" | sort -V)
sudo pacman -U --noconfirm "${pkgs[-1]}"

echo
echo "Installed. cosmic-comp is replaced at the next login; revert with:"
echo "  sudo pacman -S cosmic-comp"
