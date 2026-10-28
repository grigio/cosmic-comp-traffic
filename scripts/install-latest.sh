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
sudo pacman -U --noconfirm "$dir"/*.pkg.tar.zst

echo
echo "Installed. cosmic-comp is replaced at the next login; revert with:"
echo "  sudo pacman -S cosmic-comp"
