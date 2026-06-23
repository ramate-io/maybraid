{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/a7abebc31a8f60011277437e000eebcc01702b9f";
    rust-overlay.url = "github:oxalica/rust-overlay/02227ca8c229c968dbb5de95584cfb12b4313104";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        host = pkgs.stdenv.hostPlatform.config; # e.g., aarch64-apple-darwin

        toolchain = p: (p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          extensions = [ "rustfmt" "clippy" ];
          targets = [ host "wasm32-unknown-unknown" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain(toolchain);
        frameworks = pkgs.darwin.apple_sdk.frameworks;

        macosBlenderApp = "/Applications/Blender.app/Contents/MacOS/Blender";
        macosBlender = pkgs.writeShellScriptBin "blender" ''
          exec ${macosBlenderApp} "$@"
        '';

        # An LLVM build environment
        dependencies = with pkgs; [
          gh
          protobuf
          grpcurl
          grpcui
          ltex-ls-plus
          lychee
          uv
          perl
          llvmPackages.bintools
          openssl
          openssl.dev
          libiconv 
          pkg-config
          libclang.lib
          libz
          clang
          pkg-config
          rustPlatform.bindgenHook
          lld
          coreutils
          gcc
          rust
          wayland
          python311
        ] ++ lib.optionals stdenv.isDarwin [
          frameworks.Security
          frameworks.CoreServices
          frameworks.SystemConfiguration
          frameworks.AppKit
          frameworks.IOKit
          frameworks.CoreFoundation
          frameworks.Foundation
          libelf
          macosBlender
        ] ++ lib.optionals stdenv.isLinux [
          udev
          systemd
          bzip2
          elfutils
          jemalloc
          alsa-lib
          blender
        ];

        # Specific version of toolchain
        rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          targets = [ host "wasm32-unknown-unknown" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
    
      in {
        devShells = rec {
          default = docker-build;
          docker-build = pkgs.mkShell {
            ROCKSDB = pkgs.rocksdb;
            OPENSSL_DEV = pkgs.openssl.dev;

            hardeningDisable = ["fortify"];

            buildInputs = with pkgs; [
              # rust toolchain
              (toolchain pkgs)
            ] ++ dependencies;

            LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib/";

            shellHook = ''
              #!/usr/bin/env ${pkgs.bash}

              set -e

              # Export linker flags if on Darwin (macOS)
              if [[ "${pkgs.stdenv.hostPlatform.system}" =~ "darwin" ]]; then
                export MACOSX_DEPLOYMENT_TARGET=$(sw_vers -productVersion)
                export LDFLAGS="-L/opt/homebrew/opt/zlib/lib"
                export CPPFLAGS="-I/opt/homebrew/opt/zlib/include"

                macos_blender="${macosBlenderApp}"
                if [ ! -x "$macos_blender" ]; then
                  echo ""
                  echo "❌ Blender not found at $macos_blender"
                  echo "Install Blender 5.1.2 from https://www.blender.org/download/"
                  exit 1
                fi
                blender_version="$("$macos_blender" --version 2>/dev/null | head -1 || true)"
                if [ -z "$blender_version" ]; then
                  echo ""
                  echo "❌ Failed to run Blender at $macos_blender"
                  exit 1
                fi
                if [[ ! "$blender_version" =~ ^Blender\ 5\.1\. ]]; then
                  echo ""
                  echo "⚠️  Blender 5.1.2 is preferred for .blend → .glb export (found: $blender_version)"
                fi
              fi

              # Add ./target/debug/* to PATH
              export PATH="$PATH:$(pwd)/target/debug"

              # Add ./target/release/* to PATH
              export PATH="$PATH:$(pwd)/target/release"

              # Copy over ./githooks/pre-commit to .git/hooks/pre-commit
              cp $(pwd)/.githooks/pre-commit $(pwd)/.git/hooks/pre-commit
              chmod +x $(pwd)/.git/hooks/pre-commit

              # Include repository-local Git aliases
              git config --local include.path ../.gitconfig

              # chafa --size 30x30 --animate false --colors 8 --center true ./assets/ramate-transparent.png

              echo ""
              echo "Roadline"
              echo "Create roadmaps from Markdown."
            '';
          };
        };
      }
    );
}
