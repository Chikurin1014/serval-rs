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

          cargoLock.lockFile = ../Cargo.lock;

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

          # Resolve dioxus-desktop compilation issue: https://github.com/DioxusLabs/dioxus/issues/5203
          # patchPhase = ''
          #   cargo update -p zbus --precise 5.5.0
          #   cargo update -p zbus_macros --precise 5.5.0
          #   cargo update -p zbus_names --precise 4.2.0  # requires zvariant ^5.9.0 in 4.3.1
          #   cargo update -p zvariant --precise 5.8.0
          #   cargo update -p zvariant_derive --precise 5.8.0
          # '';
          buildPhase = ''
            cargo build --release --bin ${package-type}
          '';
          installPhase = ''
            mkdir -p $out/bin
            cp target/release/${package-type} $out/bin/serval-${package-type}
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
