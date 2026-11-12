# Cosmic Comp Traffic

A traffic monitoring and visualization tool for the Cosmic Comp desktop environment.

## Prerequisites

- [Cosmic Desktop Environment](https://cosmic.linux.freedesktop.org/)
- [GTK4](https://github.com/GTK/GTK4)
- [GStreamer](https://gstreamer.freedesktop.org/)
- [Wayland](https://wayland.freedesktop.org/)

## Building

### From Source

```bash
# Clone the repository
git clone https://github.com/cosmic-cosmic/cosmic-comp-traffic.git
cd cosmic-comp-traffic

# Install dependencies
sudo pacman -S cosmic-comp gst-plugins-base gst-plugins-good gst-plugins-bad gst-omx gtk4 cairo glib libadwaita

# Build
cargo build --release

# Run
./target/release/cosmic-comp-traffic
```

## Installation

### Nix Flake (Recommended)

This repository is a Nix flake. You can build and install it without any
additional configuration:

```bash
# Build
nix build github:cosmic-cosmic/cosmic-comp-traffic

# Inspect the binary
./result/bin/cosmic-comp

# Install to your Nix profile (requires the nix-command profile)
nix profile add github:cosmic-cosmic/cosmic-comp-traffic
```

If you prefer to build from a local clone:

```bash
git clone https://github.com/cosmic-cosmic/cosmic-comp-traffic.git
cd cosmic-comp-traffic
nix build .#cosmic-comp-traffic
./result/bin/cosmic-comp
```

### Nix DevShell (Development)

```bash
# Enter a shell with all Rust + system dependencies
nix develop

# Run tests
cargo test

# Build
cargo build --release
```

### Flatpak

```bash
# Build Flatpak package (requires the Flatpak SDK)
flatpak build -p cosmic-comp-traffic

# Install
flatpak install cosmic-comp-traffic
```

## Development

### Nix DevShell

```bash
# Enter the development environment
nix develop

# Run tests
cargo test

# Build
cargo build --release
```

## Running

```bash
./target/release/cosmic-comp-traffic
```

## Configuration

### Command-line Options

```
Usage: cosmic-comp-traffic [OPTIONS]

Options:
  -v, --verbose                Enable verbose output
  -q, --quiet                  Suppress non-error output
  -c, --config <PATH>          Path to configuration file
  -h, --help                   Print help information
  -V, --version                Print version information
```

### Configuration Files

The application reads configuration from:
- `~/.config/cosmic-comp-traffic/config.json`
- `$XDG_CONFIG_HOME/cosmic-comp-traffic/config.json`

## License

This project is licensed under the terms of the [GPL-3.0-or-later](LICENSE) license.

## Contributing

Please read the contributing guidelines before submitting a pull request. We appreciate your contributions!

### Nix Development

This project uses Nix for reproducible builds and dependency management. When contributing:

1. **Test your changes with Nix**:
   ```bash
   nix develop --command cargo test
   ```

2. **Update the flake.nix if needed**:
   - Add new dependencies to the appropriate section
   - Ensure all packages are compatible with nixpkgs

3. **Test the CI**:
   ```bash
   nix ci .#cosmic-comp-traffic
   ```

4. **Ensure the flake builds**:
   ```bash
   nix-build -A cosmic-comp-traffic
   ```

### Building and Testing

- **Regular development**:
  ```bash
  nix develop --command ./build.sh
  ```

- **Testing**:
  ```bash
  nix develop --command cargo test
  ```

- **Build for release**:
  ```bash
  nix develop --command cargo build --release
  ```

- **Run with Nix**:
  ```bash
  nix build -L .#cosmic-comp-traffic
  ```

## Credits

Built with ❤️ for the Cosmic Desktop community.
