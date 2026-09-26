# Packaging

Tree-first layouts for a signed Apple DMG, a Windows Intel zip, and a SteamOS
tarball. Scripts copy these templates into `dist/` and fill the binary plus
`maybraid/assets`.

```text
packaging/
  macos/Maybraid.app/     Info.plist, entitlements, empty MacOS + Resources
  windows/Maybraid/       sidecar + DPI manifest
  steamos/Maybraid/       .desktop + optional steam_appid.txt
  scripts/                package-macos.sh / package-windows.sh / package-steamos.sh
```

## macOS (Developer ID + notarized DMG)

```bash
packaging/scripts/package-macos.sh
open dist/Maybraid.app
```

Unsigned until `SIGN_IDENTITY` is set. After the Developer ID certificate is
in your login keychain and `notarytool store-credentials maybraid-notary` has
run:

```bash
SIGN_IDENTITY="Developer ID Application: Ramate LLC (TEAMID)" \
NOTARY_PROFILE=maybraid-notary \
  packaging/scripts/package-macos.sh
```

`SKIP_BUILD=1` reuses `target/release/maybraid`. `SKIP_NOTARY=1` signs only.
Copy `identity.local.example.md` to `identity.local.md` for Team ID notes
(gitignored).

The game reads `Contents/Resources/assets` and writes saves to
`~/Library/Application Support/io.ramate.maybraid/saves`. A checkout still
uses `maybraid/game/assets` and `.maybraid/saves`.

## Windows Intel / SteamOS

```bash
# after the matching cargo build:
packaging/scripts/package-windows.sh
packaging/scripts/package-steamos.sh
```

No OS code signing in these scripts. Archive provenance: [minisign/README.md](minisign/README.md).

## CI

[`.github/workflows/package.yml`](../.github/workflows/package.yml) packs unsigned
builds on `main`, on a published GitHub Release, on `workflow_dispatch`, and
when a commit message contains `ci-action::package`. Tag pushes are not a
trigger: publishing a release already fires `release`, and a same-ref tag
`push` used to cancel that run before assets were uploaded.

`main` and token runs upload Actions artifacts (14 days). A published release
attaches the same files to that GitHub Release. Version is
`workspace.package.version` plus a short SHA, or the release tag.

## Overrides

| Variable | Role |
|---|---|
| `MAYBRAID_ASSETS` | Bevy asset root |
| `MAYBRAID_SAVES` | `SaveRoot` directory |
| `MAYBRAID_PACKAGED` | Force user-data saves even from `cargo run` |
