Name:           cosmic-comp-traffic
Version:        %{version}
# The CI passes -D "cosmic_release N" derived from the commit count; keep a
# default for local builds. This must match the release number the publish
# step uses when pruning older assets.
%{!?cosmic_release: %define cosmic_release 1}
# Pre-release builds pass both defines so a non-tag ref (e.g. a commit SHA) can
# be used as Source0. Normal tagged builds use the defaults below.
%{!?cosmic_source_tar: %define cosmic_source_tar cosmic-comp-epoch-%{version}.tar.gz}
%{!?cosmic_source_dir: %define cosmic_source_dir cosmic-comp-epoch-%{version}}
Release:        %{cosmic_release}%{?dist}
Summary:        Compositor for the COSMIC desktop environment with macOS-style traffic-light decorations

License:        GPL-3.0-only
URL:            https://github.com/grigio/cosmic-comp-traffic
Source0:        https://github.com/pop-os/cosmic-comp/archive/refs/tags/epoch-%{version}.tar.gz#/%{cosmic_source_tar}

# Patches from the cosmic-comp-traffic fork (copied to SOURCES by the build script).
# The block below is the default for local rpmbuilds. release.yml rewrites it
# from the effective patch list (series + version overrides/extras) so no patch
# is silently left out; do not edit the marker comments.
# >>> RPM PATCH DECLARATIONS
Patch0:         0001-macos-traffic-light-decorations.patch
Patch1:         0002-wlr-foreign-toplevel-management.patch
Patch2:         0003-three-finger-workspace-swipe.patch
Patch3:         0004-alt-tab-window-cycle.patch
Patch4:         0005-wlr-gamma-control.patch
Patch5:         0006-live-theme-reload.patch
Patch6:         0007-send-frames-missing-workspace-set.patch
Patch7:         0008-output-removal-without-unwrap.patch
Patch8:         0009-fix-toplevel-handle-vram-leak.patch
Patch9:         0010-fix-dmabuf-import-feedback.patch
# <<< RPM PATCH DECLARATIONS

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  rust-packaging
BuildRequires:  gcc
BuildRequires:  libdisplay-info-devel
BuildRequires:  wayland-devel
BuildRequires:  libinput-devel
BuildRequires:  pixman-devel
BuildRequires:  mesa-libgbm-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  libseat-devel
BuildRequires:  systemd-devel
BuildRequires:  pkgconfig
BuildRequires:  just

Requires:       libdisplay-info%{?_isa}
Requires:       wayland%{?_isa}
Requires:       libinput%{?_isa}
Requires:       pixman%{?_isa}
Requires:       mesa-libgbm%{?_isa}
Requires:       libxkbcommon%{?_isa}
Requires:       libseat%{?_isa}

Provides:       cosmic-comp = %{version}-%{release}
Conflicts:      cosmic-comp
Obsoletes:      cosmic-comp < %{version}-%{release}

%description
Compositor for the COSMIC desktop environment with macOS-style traffic-light
decorations (close, minimize, maximize buttons on the left).

This is a fork of the upstream cosmic-comp from pop-os/cosmic-comp with
patches applied to provide traffic-light style window decorations.

%prep
%setup -q -n %{cosmic_source_dir}
# Explicit patch application rather than %autosetup: release.yml regenerates
# both the declarations above and this block from the same effective list.
# >>> RPM PATCH APPLICATION
%patch 0 -p1
%patch 1 -p1
%patch 2 -p1
%patch 3 -p1
%patch 4 -p1
%patch 5 -p1
%patch 6 -p1
%patch 7 -p1
%patch 8 -p1
%patch 9 -p1
# <<< RPM PATCH APPLICATION

# Use thin LTO instead of fat LTO for faster builds
sed -i 's/lto = "fat"/lto = "thin"/' Cargo.toml

# Add rpm profile for Fedora rust-packaging macros
if ! grep -q '^\[profile.rpm\]' Cargo.toml; then
  echo -e '\n[profile.rpm]\ninherits = "release"' >> Cargo.toml
fi

# Fetch cargo dependencies
cargo fetch --locked

# The cargo-build macro forces CARGO_HOME=.cargo inside the build directory,
# which would re-download everything even though the CI cache lives in
# $HOME/.cargo. Point .cargo at that cache so the downloads are reused.
if [ -d "$HOME/.cargo" ] && [ ! -e .cargo ]; then
  ln -s "$HOME/.cargo" .cargo
fi

%build
export RUSTFLAGS="${RUSTFLAGS} -C link-arg=-fuse-ld=lld"
export CARGO_BUILD_RELEASE=true
export CARGO_BUILD_FROZEN=true
%cargo_build

%install
# The RPM cargo-install macro is deliberately not used: it runs
# "cargo install --path .", which ignores Cargo.lock and re-resolves every git
# dependency to its newest commit. That pulls a libcosmic without
# CosmicTk::icon_theme and breaks the build. The cargo-build macro above already
# compiled the binary against the locked dependencies, so copy that binary
# into place directly.
install -Dm0755 target/rpm/cosmic-comp %{buildroot}%{_bindir}/cosmic-comp

# Install additional data files
install -Dm0644 data/keybindings.ron %{buildroot}%{_datadir}/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults/keybindings.ron
install -Dm0644 data/tiling-exceptions.ron %{buildroot}%{_datadir}/cosmic/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults/tiling-exceptions.ron
install -Dm0644 data/cosmic.desktop %{buildroot}%{_datadir}/wayland-sessions/cosmic.desktop
install -Dm0644 data/cosmic-session.target %{buildroot}%{_libdir}/systemd/user/cosmic-session.target
install -Dm0644 data/cosmic-session-pre.target %{buildroot}%{_libdir}/systemd/user/cosmic-session-pre.target
install -Dm0644 data/cosmic-comp.service %{buildroot}%{_libdir}/systemd/user/cosmic-comp.service
install -Dm0755 data/cosmic-service %{buildroot}%{_bindir}/cosmic-service

%files
%license LICENSE
%doc README.md
%{_bindir}/cosmic-comp
%{_bindir}/cosmic-service
%{_datadir}/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults/keybindings.ron
%{_datadir}/cosmic/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults/tiling-exceptions.ron
%{_datadir}/wayland-sessions/cosmic.desktop
%{_libdir}/systemd/user/cosmic-session.target
%{_libdir}/systemd/user/cosmic-session-pre.target
%{_libdir}/systemd/user/cosmic-comp.service

%changelog
* @@BUILD_DATE@@ Grigio <grigio.org> - %{version}-%{release}
- Initial package for Fedora