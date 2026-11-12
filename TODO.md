# TODO — cosmic-comp-traffic Nix flake

The Nix flake is the new build path. The Arch PKGBUILD + `release.yml` is the proven path.
Both can coexist; the Nix one just needs the hash grind.

## Blocker: `cargoLock.outputHashes` are all placeholders (37 of them)

Every git dep in `flake.nix` is pinned to the same dummy value:
```
"atomicwrites-0.4.2" = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
"libcosmic-1.0.0"    = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
"smithay-0.7.0"      = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
...
```

Nix fetches each git dep, hashes it, and compares against this value.
All 37 are the same wrong value. The build fails on the first git dep it fetches.

### Fix
Run `nix build .#cosmic-comp-traffic` (locally or in CI), collect the 37 real `narHash`
values from the error messages, and paste them in. One-time cost, ~10-30 min of iteration.
After that, stable until `Cargo.lock` changes.

## Secondary issues

### 1. Toolchain double-specification
`flake.nix` sets both:
```
rustc = pkgs.rustc;
cargo = pkgs.cargo;
rustToolchain = pkgs.rust-bin.stable.latest.minimal;
```
`pkgs.rustc` and `pkgs.rust-bin.stable.latest.minimal` are two different toolchain selections.
`rust-bin.stable.latest.minimal` is the `minimal` profile (rustc+cargo+std, no rust-src/clippy).
Pick one: either `pkgs.rustc`/`pkgs.cargo` (nixpkgs std) or `rust-bin.stable.latest.minimal`.
Don't mix.

### 2. `--no-default-features` claim is a lie
The comment in `flake.nix` says "We pass `--no-default-features` because the default
`systemd` feature pulls in libsystemd". But `buildRustPackage` doesn't pass
`--no-default-features` anywhere. The default feature (`systemd`) is on, so
`libsystemd`, `tracing-journald`, `logind-zbus` are all compiled in.
That's fine *if* `nativeLibs` includes them, but it's not documented.

### 3. Placeholder meta values
```
homepage = "https://github.com/your-org/cosmic-comp-traffic";
maintainers = [ pkgs.lib.maintainers.you ];
```
`your-org` and `you` are not real. Fix to `grigio` and the actual GitHub handle.

### 4. `devShells` references non-existent nixpkgs package names
```
glib, gsettings_desktop_schemas, librsvg, libjpeg, libwebp, alsa-lib, soundfont-freepats
```
`gsettings_desktop_schemas` and `soundfont-freepats` may not be real nixpkgs packages.
Verify and fix.

### 5. `nixpkgs.toml` is aspirational, not functional
The file says "This file is the single source of truth for *how* we pin Nix",
but the comment at the bottom admits:
```
# (Currently flake.nix declares its own `inputs.nixpkgs`; this file
# documents the intent and will be wired in when we migrate to
# nixpkgs-overlays or flake-parts.)
```
So `nixpkgs.toml` is currently a **dead file** — nothing reads it.
`flake.nix` has its own `inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"`.
Either wire it in (`builtins.fromTOML`) or delete it.

### 6. `flake.lock` is not derived from `nixpkgs.toml`
Pinned to `NixOS/nixpkgs@f4a03c008967841f2a0b3cb7c436fc6fc31671f6` (nixos-unstable).
Fine, but if you update the pin in `nixpkgs.toml`, you have to also update
`flake.lock` manually. The "single source of truth" claim is premature.

## CI workflow (`.github/workflows/ci.yml`)

- **`lint` job** — installs system deps, sets up Rust 1.93, runs
  `cargo check --all-targets --all-features`, `cargo clippy`, `cargo fmt --check`.
  Solid. Mirrors what a real cosmic-comp build needs.
- **`build` job** — `nix build .#cosmic-comp-traffic` with a 45-min timeout.
  This is where it will **fail** because of the `outputHashes` problem.

The gap: `lint` and `build` are independent. `lint` can pass (Rust compiles fine
under plain cargo) while `build` fails (Nix can't resolve the git deps).
That's actually *good* — it isolates the Nix-specific problem.

## Priority order

1. Fix the `outputHashes` (one real blocker).
2. Unify the toolchain.
3. Wire in `nixpkgs.toml` or delete it.
4. Fix the placeholder meta (`your-org` → `grigio`, `you` → actual handle).
5. Verify the devShell package names.
