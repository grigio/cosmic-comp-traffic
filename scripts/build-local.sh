#!/usr/bin/env bash
# Build the Arch package locally, exactly like CI does.
#
#   ./scripts/build-local.sh                 # newest upstream tag
#   ./scripts/build-local.sh 1.10.0          # specific version
#
# The package is written to ./out/. makepkg runs in a scratch directory so that
# its src/ and pkg/ folders cannot collide with the cosmic-comp source tree.
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
out_dir=$repo_dir/out
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

version=${1:-}
if [[ -z $version ]]; then
  version=$(git -C "$repo_dir" ls-remote --tags --refs https://github.com/pop-os/cosmic-comp.git 'refs/tags/epoch-*' |
    sed 's|.*refs/tags/epoch-||' | sort -V | tail -1)
fi
version=${version#epoch-}

pkgrel=$(git -C "$repo_dir" rev-list --count HEAD -- PKGBUILD patches .github/workflows/release.yml)
(( pkgrel >= 1 )) || pkgrel=1

echo "==> cosmic-comp $version, pkgrel $pkgrel"

cp "$repo_dir/PKGBUILD" "$work/"
cp -r "$repo_dir/patches" "$work/"

mkdir -p "$out_dir"
cd "$work"
COSMIC_COMP_VERSION=$version COSMIC_COMP_PKGREL=$pkgrel \
  PKGDEST=$out_dir makepkg --noconfirm --force --cleanbuild

shopt -s nullglob
for f in "$out_dir"/*.pkg.tar.zst; do
  (cd "$out_dir" && sha256sum "$(basename "$f")" > "$(basename "$f").sha256")
done
echo "==> done: $out_dir"
