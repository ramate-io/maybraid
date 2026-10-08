# Packaging

Nix is the reproducible entry point. Development (`nix develop`) stays the
broad toolchain shell. Release builds use dedicated environments so shipped
binaries do not inherit development linker paths.

```text
packaging/
  macos/Maybraid.app/     Info.plist, entitlements, empty MacOS + Resources
  windows/Maybraid/       sidecar + DPI manifest
  steamos/Maybraid/       .desktop + optional steam_appid.txt
  scripts/
    common.sh
    build-linux-sniper.sh
    package-macos.sh
    package-windows.sh
    package-steam.sh      (package-steamos.sh is a wrapper)
    package-appimage.sh
```

Rust version comes from [`rust-toolchain.toml`](../rust-toolchain.toml). Every
release `cargo` invocation uses `--locked`, an explicit target triple, the
`release` profile, and a target directory that is not `target/` from
`nix develop`.

| Platform | Entry point | Target dir | Artifact |
|---|---|---|---|
| macOS ARM64 | `nix develop .#release-macos` | `target/release-packaging` | `Maybraid-<ver>-macos-arm64.dmg` |
| Linux x86_64 | `nix develop .#release-linux` then `build-linux-sniper.sh` | `target/sniper-release` | one ELF, two packages |
| Windows x64 | `nix develop .#release-windows` or CI rustup | `target/release-packaging` | `Maybraid-<ver>-windows-x64.zip` |

Linux packages from the same sniper ELF:

- `Maybraid-<ver>-steam-linux-x64.tar.xz` — Steam depot staging
- `Maybraid-<ver>-linux-x64.AppImage` — standalone download

## macOS (Developer ID + notarized DMG)

```bash
nix develop .#release-macos --command packaging/scripts/package-macos.sh
open dist/Maybraid.app
```

The `release-macos` shell uses Xcode `clang`/`clang++` and the macOS SDK
(`xcrun --sdk macosx`). `MACOSX_DEPLOYMENT_TARGET` is **13.0**, matching
`LSMinimumSystemVersion` in the app plist — it is not taken from the build
machine's `sw_vers`. libiconv is resolved through the Apple SDK. Nix
`libiconv` search paths are not part of this environment.

Unsigned until `SIGN_IDENTITY` is set. After the Developer ID certificate is
in your login keychain and `notarytool store-credentials maybraid-notary` has
run:

```bash
SIGN_IDENTITY="Developer ID Application: Ramate LLC (TEAMID)" \
NOTARY_PROFILE=maybraid-notary \
  nix develop .#release-macos --command packaging/scripts/package-macos.sh
```

`SKIP_BUILD=1` reuses the binary already in `$CARGO_TARGET_DIR`. `SKIP_NOTARY=1`
signs only. Copy `identity.local.example.md` to `identity.local.md` for Team ID
notes (gitignored).

The packaging script inspects the staged `Contents/MacOS/maybraid` with
`otool -L` / `otool -l` and fails on `/nix/store`, Homebrew, or `/usr/local`
library references. Relocation of any bundled dylib (none today) must happen
before signing.

The game reads `Contents/Resources/assets` and writes saves to
`~/Library/Application Support/io.ramate.maybraid/saves`. A checkout still
uses `maybraid/game/assets` and `.maybraid/saves`.

Hosted macOS CI can only do a loader/window check. It does not prove Metal or
full gameplay.

## Linux (one sniper binary, two packages)

```bash
nix develop .#release-linux --command packaging/scripts/build-linux-sniper.sh
packaging/scripts/package-steam.sh
packaging/scripts/package-appimage.sh
```

`build-linux-sniper.sh` starts Valve's Steam Runtime 3.0 **sniper SDK**
container (pinned by manifest digest), installs the toolchain from
`rust-toolchain.toml` inside that container, and writes

`target/sniper-release/x86_64-unknown-linux-gnu/release/maybraid`.

Host and Nix library search paths are not mounted into the compile. GPU
drivers stay on the machine that runs the game.

### Steam depot archive

`Maybraid-<ver>-steam-linux-x64.tar.xz` is for Steam depot staging.

Extracting it does **not** install or activate Steam Linux Runtime 3.0. In
Steamworks, set the Linux launch configuration to **Steam Linux Runtime 3.0
(sniper)**.

### Standalone AppImage

`Maybraid-<ver>-linux-x64.AppImage` is the direct-download build of the **same**
executable. It does not include or start the Steam runtime.

AppImage does not remove glibc/ABI requirements. Supported standalone
baseline:

- glibc 2.31+ (Debian 11, Ubuntu 20.04, SteamOS 3, or newer)
- a host Vulkan/OpenGL driver (not bundled)

libc, libstdc++, libgcc, and GPU/display drivers are not bundled. License
notes for that choice ship in the AppImage as `THIRD_PARTY_NOTICES.txt`.

`AppRun` sets `MAYBRAID_ASSETS` from the mount and writes saves to
`~/.local/share/maybraid/saves` (or `MAYBRAID_SAVES` / `XDG_DATA_HOME`).

appimagetool is the maintained [AppImage/appimagetool](https://github.com/AppImage/appimagetool)
1.9.1 release, pinned by SHA-256.

## Windows Intel zip

```bash
# CI uses rustup from rust-toolchain.toml and CARGO_TARGET_DIR=target/release-packaging
packaging/scripts/package-windows.sh
```

No Authenticode in these scripts. Archive provenance:
[minisign/README.md](minisign/README.md).

The zip contains `Maybraid.exe`, the `assets/` sidecar, and
`README-WINDOWS.txt`. The build links the dynamic MSVC CRT; install the
[Visual C++ Redistributable 2015–2022 x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
if Windows reports a missing `VCRUNTIME140.dll`.

Saves: `%APPDATA%\Maybraid\saves`.

## CI

[`.github/workflows/package.yml`](../.github/workflows/package.yml) packs
unsigned builds on `main`, on a published GitHub Release, on
`workflow_dispatch`, and when a commit message contains `ci-action::package`.
Tag pushes are not a trigger: publishing a release already fires `release`,
and a same-ref tag `push` used to cancel that run before assets were uploaded.

Linux is compiled **once** in the sniper SDK job, then packaged for Steam and
AppImage. Caches are keyed by environment (`sniper-release`, `release-macos`,
`release-windows`), target, and lockfile.

`main` and token runs upload Actions artifacts (14 days). A published release
attaches the same files to that GitHub Release. Version is
`workspace.package.version` plus a short SHA, or the release tag.

Scripts run the same dependency checks locally as in CI. Smoke tests launch
from `/tmp` so asset paths cannot depend on the repo cwd. A process that
stays up is a **loader** check only; CI does not claim full GPU/runtime
success. Missing-library errors fail the job.

## Overrides

| Variable | Role |
|---|---|
| `MAYBRAID_ASSETS` | Bevy asset root |
| `MAYBRAID_SAVES` | `SaveRoot` directory |
| `MAYBRAID_PACKAGED` | Force user-data saves even from `cargo run` |
| `MAYBRAID_LINUX_BIN` | Override the sniper ELF used by Linux packagers |
| `SNIPER_IMAGE` | Override the pinned sniper SDK image |
| `CARGO_TARGET_DIR` | Release target directory (set by the flake shells) |
