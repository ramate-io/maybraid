# Packaging

Declared builds and small assembly scripts. `nix develop` is the development
shell only.

| Responsibility | Owner |
|---|---|
| macOS compile | `nix build .#maybraid-macos` (Crane + selected Xcode) |
| Linux compile | `packaging/linux/Dockerfile` (pinned sniper SDK + pinned Rust) |
| AppImage libs | [linuxdeploy](https://docs.appimage.org/packaging-guide/from-source/linuxdeploy-user-guide.html) |
| macOS DMG | `packaging/scripts/package-macos.sh` (Apple tools) |
| Windows zip | native MSVC `cargo` + `package-windows.sh` |
| Orchestration | `.github/workflows/package.yml` |

Version is `packaging/version.sh`. Rust channel is `rust-toolchain.toml`.
Dependency checks: `packaging/validate.sh`. Startup: `packaging/smoke.sh`.

## macOS ARM64

Needs **Xcode 15+** at `$MAYBRAID_XCODE` (default `/Applications/Xcode.app`).
Metal (`xcrun metal`) is that Xcode, not Nix. The build is impure:

```bash
nix build .#maybraid-macos --impure --option sandbox false
BINARY=result/bin/maybraid packaging/scripts/package-macos.sh
packaging/validate.sh macho dist/Maybraid.app/Contents/MacOS/maybraid
```

`MACOSX_DEPLOYMENT_TARGET` is 13.0 (same as `LSMinimumSystemVersion`).
libiconv comes from the Apple SDK. `dontFixup` keeps Nix from rewriting rpaths.

Unsigned unless `SIGN_IDENTITY` is set. See `identity.local.example.md`.

## Linux x86_64 (one ELF, two packages)

```bash
nix run .#build-linux
# or: packaging/scripts/build-linux.sh
packaging/scripts/package-steam.sh
packaging/scripts/package-appimage.sh
```

The Dockerfile installs Rust at image-build time. `build-linux.sh` is one
`docker build` plus one `cargo build` in that image. Host/Nix library paths
are not passed in.

- `Maybraid-<ver>-steam-linux-x64.tar.xz` — Steam depot staging. Steamworks
  must launch with **Steam Linux Runtime 3.0 (sniper)**. Extracting the
  archive does not install that runtime.
- `Maybraid-<ver>-linux-x64.AppImage` — same ELF. linuxdeploy runs *inside*
  the sniper SDK so bundled libs match the compile. libc/libstdc++/GPU
  drivers stay on the host. Baseline: glibc 2.31+ (Ubuntu 20.04 / Debian 11
  / SteamOS 3).

## Windows x64

```bash
cargo build -p maybraid --release --locked --target x86_64-pc-windows-msvc
packaging/scripts/package-windows.sh
```

Dynamic MSVC CRT: install the
[VC++ 2015–2022 x64 redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
if `VCRUNTIME140.dll` is missing.

## CI

[`.github/workflows/package.yml`](../.github/workflows/package.yml) runs on
`main`, published releases, `workflow_dispatch`, and `ci-action::package`.
One `version` job; Linux compiles once; smoke/validate are separate steps.

| Variable | Role |
|---|---|
| `MAYBRAID_ASSETS` | Bevy asset root |
| `MAYBRAID_SAVES` | Save directory |
| `MAYBRAID_PACKAGED` | Force user-data saves |
| `MAYBRAID_XCODE` | Xcode `.app` for release builds |
| `MAYBRAID_LINUX_BIN` | Override the sniper ELF |
