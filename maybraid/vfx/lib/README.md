# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. First delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `firey_explosion`.

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Flipbook cards remain the distant / wisp layer. Spawn through `VfxLibrary` and `Commands::spawn_vfx`. Scale is spatial and applied once at init. Intensity scales particle count and emission. Playback speeds the authored layer envelope without changing those ratios. Shared Hanabi assets are never mutated per instance.

```text
cargo test -p maybraid-vfx
cargo run -p vfx-playground
```
