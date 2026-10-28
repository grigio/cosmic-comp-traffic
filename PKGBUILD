# Maintainer: grigio <grigio.org>
#
# cosmic-comp with the macOS traffic-light decorations patch applied.
# Derived from Arch Linux's official cosmic-comp PKGBUILD.
#
# The version is normally injected by the CI workflow, e.g.
#   COSMIC_COMP_VERSION=1.11.0 makepkg -si
# and defaults to the last version this file was written against.
#
# Patches are applied in this order:
#   patches/common/*.patch      (patches that work for every supported tag)
#   patches/<pkgver>/*.patch    (version-specific extras, if the directory exists)

pkgname=cosmic-comp-traffic
pkgver=${COSMIC_COMP_VERSION:-1.11.0}
pkgrel=${COSMIC_COMP_PKGREL:-1}
epoch=1
pkgdesc='Compositor for the COSMIC desktop environment with macOS-style traffic-light decorations'
arch=(x86_64)
url='https://github.com/grigio/cosmic-comp-traffic'
license=(GPL-3.0-only)
groups=(cosmic)
depends=(
  fontconfig
  libgcc
  glibc
  libdisplay-info
  libseat.so
  libinput
  libxcb
  libxkbcommon
  mesa
  pixman
  systemd-libs
  wayland
)
makedepends=(
  cargo
  git
  just
  lld
)
# Same binary, same paths as the official package, so it is a drop-in
# replacement: installing this one removes cosmic-comp.
provides=(cosmic-comp)
conflicts=(cosmic-comp)
replaces=(cosmic-comp)
source=("cosmic-comp::git+https://github.com/pop-os/cosmic-comp.git#tag=epoch-${pkgver}")

_patch_dirs() {
  printf '%s\n' "$startdir/patches/common"
  if [[ -d "$startdir/patches/$pkgver" ]]; then
    printf '%s\n' "$startdir/patches/$pkgver"
  fi
}

prepare() {
  cd cosmic-comp

  cargo fetch --locked

  local dir patch applied=0
  while read -r dir; do
    for patch in "$dir"/*.patch; do
      [[ -e "$patch" ]] || continue
      msg2 "Applying $(basename "$patch")"
      git apply --whitespace=nowarn -p1 "$patch"
      applied=$((applied + 1))
    done
  done < <(_patch_dirs)

  if ((applied == 0)); then
    error "No patches found for $pkgver"
    return 1
  fi
  msg2 "Applied $applied patch(es)"

  # fat LTO costs a lot of build time for no measurable gain here.
  sed 's/lto = "fat"/lto = "thin"/' -i Cargo.toml
}

build() {
  cd cosmic-comp
  RUSTFLAGS+=" -C link-arg=-fuse-ld=lld"
  just build-release --frozen
}

package() {
  cd cosmic-comp
  just rootdir="${pkgdir}" install
}
