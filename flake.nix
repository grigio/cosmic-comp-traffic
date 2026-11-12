{
  description = "Compositor for the COSMIC desktop environment with traffic-light decorations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    crane.url = "github:ipetkov/crane/63396562b8e08efda3b3c66e32661b8a513055de";
    crane.inputs.nixpkgs.follows = "nixpkgs";

    rust.url = "github:oxalica/rust-overlay";
    rust.inputs.nixpkgs.follows = "nixpkgs";

    nix-filter.url = "github:numtide/nix-filter";
    nix-filter.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    parts,
    crane,
    rust,
    nix-filter,
    ...
  }:
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
        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
        craneArgs = {
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
            ];
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            autoPatchelfHook
            cmake
            just
            libdisplay-info
          ];

          buildInputs = with pkgs; [
            wayland
            systemd # For libudev
            seatd # For libseat
            libxkbcommon
            libinput
            mesa # For libgbm
            fontconfig
            stdenv.cc.cc.lib
            pixman
            libdisplay-info
            dbus
            glib
            gsettings-desktop-schemas
            librsvg
            libpng
            libjpeg_turbo
            libwebp
            alsa-lib
            freepats
            libXi
            libXrandr
            libXfixes
            libXext
            libXdamage
            libXcomposite
            libXrender
            libXxf86vm
            libxcb
            vulkan-loader
          ];

          propagatedBuildInputs = with pkgs; [
            libdisplay-info
          ];

          runtimeDependencies = with pkgs; [
            libglvnd # For libEGL
            wayland # winit->wayland-sys wants to dlopen libwayland-egl.so
            # for running in X11
            libx11
            libxcursor
            libxcb
            libxi
            libxkbcommon
            # for vulkan backend
            vulkan-loader
          ];

          doCheck = false;
          strictDeps = false;
        };

        cosmic-comp-traffic = craneLib.buildPackage craneArgs;
      in {
        apps.cosmic-comp-traffic = {
          type = "app";
          program = lib.getExe' self'.packages.default "cosmic-comp";
        };

        checks.cosmic-comp-traffic = cosmic-comp-traffic;
        packages.default = cosmic-comp-traffic;

        devShells.default = craneLib.devShell {
          LD_LIBRARY_PATH = lib.makeLibraryPath (
            __concatMap (d: d.runtimeDependencies) (__attrValues self'.checks)
          );

          # include build inputs
          inputsFrom = [cosmic-comp-traffic];
        };
      };
    };
}
