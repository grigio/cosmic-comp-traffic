{
  description = "Compositor for the COSMIC desktop environment with traffic-light decorations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    rust.url = "github:oxalica/rust-overlay";
    rust.inputs.nixpkgs.follows = "nixpkgs";

    nix-filter.url = "github:numtide/nix-filter";
    nix-filter.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    parts,
    rust,
    nix-filter,
    ...
  }: let
    # Allow overriding cargoVendorHash from file (for CI)
    cargoVendorHash =
      if builtins.pathExists ./cargoVendorHash.nix
      then (import ./cargoVendorHash.nix)
      else null;
  in
    parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      perSystem = {
        self',
        lib,
        system,
        ...
      }: let
        pkgs = nixpkgs.legacyPackages.${system}.extend rust.overlays.default;

        # Use libdisplay-info_0_3 like nixpkgs cosmic-comp package
        libdisplay-info = pkgs.libdisplay-info_0_3;

        # System dependencies needed at build time for pkg-config
        sysDeps = with pkgs; [
          libdisplay-info
          wayland
          libgbm
          libinput
          pixman
          seatd
          udev
          libxkbcommon
        ];

        # PKG_CONFIG_PATH for libdisplay-info
        libdisplay-info-pkgconfig = builtins.toString (libdisplay-info.outPath + "/lib/pkgconfig");

        # Use rustPlatform.buildRustPackage like nixpkgs cosmic-comp package
        cosmic-comp-traffic = pkgs.rustPlatform.buildRustPackage {
          pname = "cosmic-comp-traffic";
          version = self.rev or "dirty";

          src = nix-filter.lib.filter {
            root = ./.;
            include = [
              ./src
              ./cosmic-comp-config
              ./data
              ./resources
              ./i18n.toml
              ./Cargo.toml
              ./Cargo.lock
              ./rust-toolchain.toml
              ./rustfmt.toml
              ./build.rs
              ./justfile
              ./cargo.just
            ];
          };

          cargoHash = "sha256-zIckNtQgphFq/mbWJNE+Am5iWxhKViUT7DKBiy9Jwfs=";

          nativeBuildInputs = with pkgs; [
            pkg-config
            just
          ];

          buildInputs = sysDeps;

          # Set PKG_CONFIG_PATH for cargo build
          preConfigure = ''
            export PKG_CONFIG_PATH="${libdisplay-info-pkgconfig}:$PKG_CONFIG_PATH"
          '';

          # Use just for building
          dontUseJustBuild = true;
          dontUseJustCheck = true;

          justFlags = [
            "--set"
            "prefix"
            (placeholder "out")
            "--set"
            "cargo-target-dir"
            "target/${pkgs.stdenv.hostPlatform.rust.cargoShortTarget}"
          ];

          doCheck = false;
        };
      in {
        apps.cosmic-comp-traffic = {
          type = "app";
          program = lib.getExe' self'.packages.default "cosmic-comp";
        };

        checks.cosmic-comp-traffic = cosmic-comp-traffic;
        packages.default = cosmic-comp-traffic;

        devShells.default = pkgs.mkShell {
          buildInputs = [ cosmic-comp-traffic ] ++ sysDeps;
          LD_LIBRARY_PATH = lib.makeLibraryPath sysDeps;
        };
      };
    };
}