{
  ...
}:

{
  perSystem =
    { config, pkgs, ... }:
    let
      package-types = [ "web" ];
      package-names = [ "default" ] ++ package-types;
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
      mkPackage =
        package-type:
        rustPlatform.buildRustPackage {
          pname = "serval-${package-type}";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ../.;

          cargoLock = {
            lockFile = ../Cargo.lock;
            # Git dependencies have no checksum in Cargo.lock, so Nix needs their hashes.
            # Both crates come from the same checkout of DioxusLabs/components.
            # Update these whenever that revision changes in Cargo.lock.
            outputHashes = {
              "dioxus-attributes-0.1.0" = "sha256-y4aeyIKJxwdi4L+D9Gm+jM5eXaBGZCKfcliyoYnypE4=";
              "dioxus-primitives-0.0.1" = "sha256-y4aeyIKJxwdi4L+D9Gm+jM5eXaBGZCKfcliyoYnypE4=";
            };
          };

          # ref: https://github.com/DioxusLabs/dioxus/blob/main/flake.nix
          buildInputs =
            with pkgs;
            [
              openssl
              libiconv
              pkg-config
            ]
            ++ lib.optionals pkgs.stdenv.isLinux [
              glib
              gtk3
              libsoup_3
              libudev-zero
              webkitgtk_4_1
              xdotool
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              apple-sdk
              libiconv
            ];
          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
            dioxus-cli
            wasm-bindgen-cli_0_2_126
          ];
          buildPhase = ''
            dx build --release --package ${package-type}
          '';
          installPhase = ''
            mkdir -p $out/bin
            cp -r target/release/dx/${package-type} $out/bin/serval-${package-type}
          '';
          doCheck = false; # Disable tests to avoid building deps for them
        };
    in
    {
      packages = {
        default = mkPackage (builtins.head package-types);
      }
      // builtins.listToAttrs (
        map (name: {
          inherit name;
          value = mkPackage name;
        }) package-types
      );

      devShells = builtins.listToAttrs (
        map (name: {
          inherit name;
          value = pkgs.mkShell {
            inputsFrom = [ config.packages.${name} ];
          };
        }) package-names
      );
    };
}
