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
            # Development shell only — release-macos does not set these.
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

          # Dedicated macOS release shell: pinned Rust + Apple SDK only.
          # mkShellNoCC avoids the Nix clang wrapper. libiconv is resolved
          # through the macOS SDK, not pkgs.libiconv.
          release-macos = pkgs.mkShellNoCC {
            packages = [
              releaseRust
            ];

            # Matches packaging/macos Info.plist LSMinimumSystemVersion.
            # Do not derive this from `sw_vers` on the build machine.
            MACOSX_DEPLOYMENT_TARGET = "13.0";
            CARGO_TARGET_DIR = "target/release-packaging";

            shellHook = ''
              #!/usr/bin/env ${pkgs.bash}
              set -e

              unset NIX_LDFLAGS NIX_CFLAGS_COMPILE NIX_CC LIBRARY_PATH CPATH
              unset C_INCLUDE_PATH CPLUS_INCLUDE_PATH PKG_CONFIG_PATH

              if [ -d /Applications/Xcode.app/Contents/Developer ]; then
                export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
              fi
              if sdkroot="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null)" && [ -d "$sdkroot" ]; then
                export SDKROOT="$sdkroot"
                export CFLAGS="-isysroot $sdkroot -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
                export CXXFLAGS="-isysroot $sdkroot -stdlib=libc++ -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
                export LDFLAGS="-isysroot $sdkroot -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
                export CC="$(xcrun --find clang)"
                export CXX="$(xcrun --find clang++)"
                export AR="$(xcrun --find ar)"
                # rustc always passes -liconv on Darwin. Prefer the Apple SDK,
                # then /usr/lib, so a Nix rustc wrapper cannot put
                # /nix/store/.../libiconv first. Runtime path stays
                # /usr/lib/libiconv.2.dylib (Apple system library).
                export RUSTFLAGS="-L native=$sdkroot/usr/lib -L native=/usr/lib"
              else
                echo "❌ macOS SDK not found via xcrun --sdk macosx" >&2
                exit 1
              fi

              echo ""
              echo "macOS release build environment (Apple SDK, no Nix CC)"
              echo "Target: aarch64-apple-darwin (ARM64)"
              echo "Deployment target: $MACOSX_DEPLOYMENT_TARGET"
              echo "Build dir: $CARGO_TARGET_DIR"
              echo "SDKROOT: $SDKROOT"
              echo "Compiler: $CC"
            '';
          };

          # Linux release entry point: launches the sniper SDK container.
          # No host/Nix library paths — the SDK image owns compilation.
          release-linux = pkgs.mkShellNoCC {
            packages = [];
            CARGO_TARGET_DIR = "target/sniper-release";
            shellHook = ''
              unset LIBRARY_PATH LD_LIBRARY_PATH CPATH C_INCLUDE_PATH CPLUS_INCLUDE_PATH
              unset PKG_CONFIG_PATH NIX_LDFLAGS NIX_CFLAGS_COMPILE NIX_CC
              echo ""
              echo "Linux release entry point (Steam Runtime 3.0 sniper SDK)"
              echo "Build:     packaging/scripts/build-linux-sniper.sh"
              echo "Steam:     packaging/scripts/package-steam.sh"
              echo "AppImage:  packaging/scripts/package-appimage.sh"
              echo "Build dir: $CARGO_TARGET_DIR"
            '';
          };

          # Dedicated Windows release shell (for local builds; CI uses native rustup)
          release-windows = pkgs.mkShellNoCC {
            packages = [
              releaseRust
            ];
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
