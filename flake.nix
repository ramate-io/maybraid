{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/8c50a710ddca43d7a530fb805ad55bde8d0141c5";
    rust-overlay.url = "github:oxalica/rust-overlay/e7a078c7feb51f37955a832b22a96de5fccb1f7a";
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
        pre_host = pkgs.stdenv.hostPlatform.config; # e.g. arm64-apple-darwin on Apple Silicon
        host = pkgs.lib.replaceStrings [ "arm64-" ] [ "aarch64-" ] pre_host;

        toolchain = p: (p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          extensions = [ "rustfmt" "clippy" ];
          targets = [ host "wasm32-unknown-unknown" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain(toolchain);

        macosBlenderApp = "/Applications/Blender.app/Contents/MacOS/Blender";
        macosBlender = pkgs.writeShellScriptBin "blender" ''
          exec ${macosBlenderApp} "$@"
        '';

        # An LLVM build environment
        dependencies = with pkgs; [
          tracy
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
          python311
        ] ++ lib.optionals stdenv.isDarwin [
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
          wayland
        ];

        # Specific version of toolchain
        rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          targets = [ host "wasm32-unknown-unknown" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
    
        # Minimal release toolchain: pinned Rust from rust-toolchain.toml, no dev tools
        releaseRust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);

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

            # rustc's Darwin target always passes `-liconv`. The Nix `cc`
            # wrapper does not reliably inject `libiconv` into rustc's own
            # link line (and Xcode `DEVELOPER_DIR` can hide the SDK copy).
            # For development builds, add Nix libiconv to ensure successful linking.
            # For release/packaging builds, the packaging script unsets these to use
            # the system libiconv and avoid /nix/store paths in shipped binaries.
            RUSTFLAGS = pkgs.lib.optionalString pkgs.stdenv.isDarwin
              "-L native=${pkgs.libiconv}/lib";
            LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isDarwin
              "${pkgs.libiconv}/lib";

            shellHook = ''
              #!/usr/bin/env ${pkgs.bash}

              set -e

              # Export linker flags if on Darwin (macOS)
              if [[ "${pkgs.stdenv.hostPlatform.system}" =~ "darwin" ]]; then
                export MACOSX_DEPLOYMENT_TARGET=$(sw_vers -productVersion)
                export LDFLAGS="-L${pkgs.libiconv}/lib -L/opt/homebrew/opt/zlib/lib''${LDFLAGS:+ $LDFLAGS}"
                export CPPFLAGS="-I/opt/homebrew/opt/zlib/include''${CPPFLAGS:+ $CPPFLAGS}"

                # mistral.rs Metal kernel precompile needs `xcrun metal`.
                # udpipe-rs compiles vendored C++ through Nix `clang++`, which
                # does not add `-isysroot` on its own and then cannot find
                # libc++ headers (`<cstring>`, `<cstddef>`). Prefer full Xcode
                # over the Nix apple-sdk, which also lacks `metal`.
                if [ -d /Applications/Xcode.app/Contents/Developer ]; then
                  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
                  if sdkroot="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null)" && [ -d "$sdkroot" ]; then
                    export SDKROOT="$sdkroot"
                    export CFLAGS="-isysroot $sdkroot''${CFLAGS:+ $CFLAGS}"
                    export CXXFLAGS="-isysroot $sdkroot -stdlib=libc++''${CXXFLAGS:+ $CXXFLAGS}"
                  fi
                fi

                macos_blender="${macosBlenderApp}"
                if [ ! -x "$macos_blender" ]; then
                  echo ""
                  echo "⚠️  Blender not found at $macos_blender"
                  echo "   Install Blender 5.1.2 from https://www.blender.org/download/ for .blend export."
                else
                  blender_version="$("$macos_blender" --version 2>/dev/null | head -1 || true)"
                  if [ -z "$blender_version" ]; then
                    echo ""
                    echo "⚠️  Failed to run Blender at $macos_blender"
                  elif [[ ! "$blender_version" =~ ^Blender\ 5\.1\. ]]; then
                    echo ""
                    echo "⚠️  Blender 5.1.2 is preferred for .blend → .glb export (found: $blender_version)"
                  fi
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

              echo ""
              echo "Maybraid"
              echo "A game of peer-based state."
            '';
          };

          # Dedicated macOS release shell: minimal no-cc shell with only Rust
          # Let Xcode's clang/SDK come through naturally for C/C++ dependencies
          release-macos = pkgs.mkShellNoCC {
            packages = [
              releaseRust
            ];

            # Set minimum macOS version explicitly (supports 11.0+)
            MACOSX_DEPLOYMENT_TARGET = "11.0";

            # Use separate target directory for release builds
            CARGO_TARGET_DIR = "target/release-packaging";

            shellHook = ''
              #!/usr/bin/env ${pkgs.bash}
              set -e

              # Configure Xcode toolchain for both Rust linking and C/C++ dependencies
              if [ -d /Applications/Xcode.app/Contents/Developer ]; then
                export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
                if sdkroot="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null)" && [ -d "$sdkroot" ]; then
                  export SDKROOT="$sdkroot"
                  # Explicit flags for cc-rs and build.rs C/C++ compilation
                  export CFLAGS="-isysroot $sdkroot -mmacosx-version-min=11.0"
                  export CXXFLAGS="-isysroot $sdkroot -stdlib=libc++ -mmacosx-version-min=11.0"
                  export LDFLAGS="-isysroot $sdkroot -mmacosx-version-min=11.0 -L/usr/lib"
                  
                  # Point compilers at Xcode's toolchain
                  export CC="$(xcrun --find clang)"
                  export CXX="$(xcrun --find clang++)"
                  export AR="$(xcrun --find ar)"
                  
                  # Force Rust linker to use system libiconv instead of Nix
                  # Override any Nix-injected library paths
                  export RUSTFLAGS="-L/usr/lib -C link-arg=-liconv"
                fi
              fi

              echo ""
              echo "macOS release build environment (no Nix CC)"
              echo "Target: aarch64-apple-darwin (ARM64)"
              echo "Deployment target: $MACOSX_DEPLOYMENT_TARGET"
              echo "Build dir: $CARGO_TARGET_DIR"
              echo "Compiler: \${CC:-system}"
            '';
          };

          # Dedicated Windows release shell (for local builds; CI uses native toolchain)
          release-windows = pkgs.mkShell {
            buildInputs = [
              releaseRust
            ];

            # Use separate target directory for release builds
            CARGO_TARGET_DIR = "target/release-packaging";

            shellHook = ''
              echo ""
              echo "Windows release build environment"
              echo "Target: x86_64-pc-windows-msvc"
              echo "Build dir: $CARGO_TARGET_DIR"
            '';
          };
        };
      }
    );
}
