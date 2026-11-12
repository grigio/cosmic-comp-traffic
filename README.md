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

### Arch Linux (AUR)

Packages are built automatically for each upstream release and published to GitHub Releases. Install with:

```bash
# Download latest release
gh release download -R grigio/cosmic-comp-traffic -p 'cosmic-comp-traffic-*.pkg.tar.zst*' -D /tmp/cosmic-comp-traffic
cd /tmp/cosmic-comp-traffic && sha256sum -c ./*.pkg.tar.zst.sha256
sudo pacman -U ./*.pkg.tar.zst
```

The package is called `cosmic-comp-traffic`, provides/conflicts/replaces `cosmic-comp`, and installs the same files to the same paths.

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

### CI Pipeline

The CI runs:
- **format** - `cargo fmt --all -- --check`
- **clippy-check** - `cargo clippy --all-features -- -D warnings`
- **check-features** - `cargo check` with various feature combinations
- **test** - `cargo test --all-features` (with libseat built from source)
- **nix-build** - `nix build .#default`

## Configuration

The compositor uses COSMIC's standard configuration system. See [cosmic-comp](https://github.com/pop-os/cosmic-comp) for configuration details.

## Upstream

This fork tracks [pop-os/cosmic-comp](https://github.com/pop-os/cosmic-comp). Patches are maintained in the `patches/` directory and applied automatically during build.

## License

GPL-3.0-or-later - see [LICENSE](LICENSE)

Based on COSMIC Comp by System76/Pop!_OS, licensed under GPL-3.0-only.