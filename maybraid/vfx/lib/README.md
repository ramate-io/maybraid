# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. First delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `fiery_explosion`. Milestone 2.1 dust playback is a separate opt-in path: [`DustBundle`](src/dust/assets.rs) loads the authored assets, [`DustPlayback`](src/dust/playback.rs) / [`DustDisturbance`](src/dust/disturbance.rs) / [`DustBounds`](src/dust/bounds.rs) are the runtime uniforms and contract, and [`DustPlaybackPlugin`](src/dust/material.rs) registers the embedded shader. The playground example `matched_dust` consumes those APIs.

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Flipbook cards remain the distant / wisp layer. Spawn through `VfxLibrary` and `Commands::spawn_vfx`. Scale is spatial and applied once at init. Intensity scales particle count and emission. Playback speeds the authored layer envelope without changing those ratios. `ExplosionPalette` recolors mesh, particles, and light; `tint` multiplies that palette. Flipbooks store heat and shape, not baked orange. An omitted seed is generated; an explicit seed reproduces sampled lobe variation. Shared Hanabi assets are never mutated per instance.

```text
cargo test -p maybraid-vfx
cargo run -p vfx-playground
cargo run -p vfx-playground --example matched_dust -- C --view orbit --inspect-time 0.7
```
