# cosmic-comp-traffic

A fork of [pop-os/cosmic-comp](https://github.com/pop-os/cosmic-comp) with macOS-style traffic-light window decorations and additional patches.

This is the COSMIC compositor with traffic-light close/minimize/maximize buttons on window decorations, plus several quality-of-life patches.

## Patches Applied

- **macOS traffic-light decorations** - Close/minimize/maximize buttons on window titlebars
- **wlr-foreign-toplevel-management** - Better window management integration
- **Three-finger workspace swipe** - Touchpad gesture for workspace switching
- **Alt-tab window cycling** - Improved window cycling behavior
- **wlr-gamma-control** - Gamma control protocol support
- **Live theme reload** - Theme changes apply without restart
- **Frame callback fixes** - Missing workspace set handling
- **Output removal fixes** - Avoid unwrap panics on output removal
- **Toplevel handle lifetime fixes** - Weak references avoid pinning closed windows and their GPU textures
- **DMABUF import feedback fixes** - Stop advertising formats the compositor cannot import

`patches/series` defines the common patch order. Every build uses the *effective*
list derived from it: series entries with a same-named `patches/<version>/`
override substituted, plus any version-only extras. For `epoch-1.11.0` that also
includes `0009-clippy-needless-bool.patch`. Arch, RPM and the published release
notes are all generated from that same list, so no patch is silently omitted.

Support for `epoch-1.10.0` was dropped: packages are built for `epoch-1.11.0`
and newer tags, plus pre-releases built from upstream master.

The Nix flake and plain `cargo build` use the in-tree `src/` snapshot, which is
kept in sync with the common series by `scripts/check-src-sync.sh` (also run in
CI). To check that the patches apply to every supported tag, run
`./scripts/check-patches.sh` (add `--offline` to skip fetching upstream tags,
`--ref <rev>` to test an arbitrary ref, `--fmt` for formatting and
`--include-rc` for fork rc tags).

## Prerequisites

- Rust toolchain (managed via rust-toolchain.toml)
- System dependencies (see flake.nix or CI for full list)

## Building

### Nix Flake (Recommended)

This repository is a Nix flake. You can build and run it directly:

```bash
# Build from GitHub
nix build github:grigio/cosmic-comp-traffic

# Run the compositor
./result/bin/cosmic-comp
```

Or from a local clone:

```bash
git clone https://github.com/grigio/cosmic-comp-traffic.git
cd cosmic-comp-traffic
nix build .#default
./result/bin/cosmic-comp
```

### Nix DevShell (Development)

```bash
# Enter a shell with all Rust + system dependencies
nix develop

# Inside the shell:
cargo test          # Run tests
cargo build --release  # Build release
./target/release/cosmic-comp  # Run
```

### Cargo (Without Nix)

```bash
# Install system dependencies first (see CI for package list)
# Then:
cargo build --release
./target/release/cosmic-comp
```

## Installation

### Nix Profile

```bash
nix profile add github:grigio/cosmic-comp-traffic
```

### NixOS Module

Add to your `configuration.nix`:

```nix
inputs.cosmic-comp-traffic.url = "github:grigio/cosmic-comp-traffic";

{ config, pkgs, ... }:
{
  programs.cosmic-comp-traffic.enable = true;
}
```

### Arch Linux

Packages are built automatically for each upstream release and published to GitHub Releases. Install with:

```bash
# Download latest release
gh release download -R grigio/cosmic-comp-traffic -p 'cosmic-comp-traffic-[0-9]*.pkg.tar.zst' -D /tmp/cosmic-comp-traffic
sudo pacman -U /tmp/cosmic-comp-traffic/*.pkg.tar.zst
```

The package is called `cosmic-comp-traffic`, provides/conflicts/replaces `cosmic-comp`, and installs the same files to the same paths.

### Fedora

RPM packages are also built automatically for each upstream release and published to GitHub Releases. Install with:

```bash
# Download latest release
gh release download -R grigio/cosmic-comp-traffic -p 'cosmic-comp-traffic-[0-9]*.rpm' -D /tmp/cosmic-comp-traffic
sudo dnf install /tmp/cosmic-comp-traffic/*.rpm
```

The package is called `cosmic-comp-traffic`, provides/conflicts/obsoletes `cosmic-comp`, and installs the same files to the same paths.

## Development

### Nix DevShell

```bash
# Enter the development environment
nix develop

# Run tests
cargo test

# Build
cargo build --release

# Run clippy
cargo clippy --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

### Running Tests

```bash
nix develop --command cargo test
```

### Maintenance

- Bump `[package].version` in `nixpkgs.toml` when the supported upstream epoch
  changes, and regenerate `flake.lock` whenever flake inputs change.

### CI Pipeline

The CI runs:
- **patch-sync** - `scripts/check-src-sync.sh` verifies `src/` matches the common series
- **format** - `cargo fmt --all -- --check`
- **clippy-check** - `cargo clippy --all-features -- -D warnings`
- **check-features** - `cargo check` with various feature combinations
- **test** - `cargo test --all-features` (with libseat built from source)
- **nix-build** - `nix build .#default` (uses the hash in `cargoVendorHash.nix`)

The release workflow computes the effective patch list once and rewrites the
RPM spec's `PatchN:` / `%patchN` blocks and the release notes from it. RPM
patches are no longer declared by hand in the spec. Arch `pkgrel` and RPM
`Release` come from the single `release-counter` file (CI fails if it is not
greater than the highest published build); bump that file when a rebuild is
needed. Before building a pre-release from an upstream ref, the workflow also
runs `scripts/check-patches.sh --ref` against that ref.

## Configuration

The compositor uses COSMIC's standard configuration system. See [cosmic-comp](https://github.com/pop-os/cosmic-comp) for configuration details.

## Upstream

This fork tracks [pop-os/cosmic-comp](https://github.com/pop-os/cosmic-comp). Patches are maintained in the `patches/` directory and applied automatically during build.

## License

GPL-3.0-or-later - see [LICENSE](LICENSE)

Based on COSMIC Comp by System76/Pop!_OS, licensed under GPL-3.0-only.