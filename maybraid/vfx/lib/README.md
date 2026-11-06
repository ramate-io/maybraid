# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. First delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `fiery_explosion`.

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Flipbook cards remain the distant / wisp layer. Spawn typed recipes with `commands.spawn_scene(Effect { recipe: Dust { .. }, spawn })`, or the flattened `EffectDefinition` path through `VfxLibrary` and `spawn_vfx`. Scale is spatial and applied once at init. Intensity scales particle count and emission. Playback speeds the authored layer envelope without changing those ratios. `ExplosionPalette` recolors mesh, particles, and light; `tint` multiplies that palette. Flipbooks store heat and shape, not baked orange. An omitted seed is generated; an explicit seed reproduces sampled lobe variation. Shared Hanabi assets are never mutated per instance.

```text
cargo test -p maybraid-vfx
cargo run -p vfx-playground
```

## Packaged dust

`VfxLibrary::dust` is the shared eight-view baked dust recipe. It lowers through
the same layer composition, spatial scale, playback controls, instance ownership,
and cancellation/cleanup as other effects. `commands.spawn_scene(Effect { recipe, spawn })`
retains `Dust` on the root for typed queries. Each instance owns its material
clock; all instances share the KTX2 texture. `intensity` adjusts this baked
layer's brightness rather than spawning extra particles.

`PackagedVfxPlugin` registers typed `PackagedFlipbook` and `StaticVolume` loaders
for `.flipbook.json` and `.volume.json`. `VfxPlugin` includes it automatically.
Images load through Bevy's standard KTX2 decoder with linear clamp sampling;
their manifest-relative paths and handles remain normal AssetServer
dependencies. Configure the application's asset root to `maybraid/assets`, as
the game and playground already do. Await recursive dependency loading before
using a manifest's image. The dust layer cancels its instance if loading fails.

Use a camera with Bevy's `DepthPrepass` for independent scene-depth rejection
and soft contact fading of each captured plane; the playground enables it.
Without that prepass, the material uses an approximate blended depth. The bake
was authored at 20° elevation and supplies no scene lighting response or
volumetric simulation. Cold GPU pipeline compilation may skip opening frames;
warm the effect before comparing a precise authored timestamp.

See [asset metadata and provenance](../../assets/vfx/dust/README.md),
[offline export tools](../../../bin/vfx/README.md), and
[playground controls](../playground/README.md).
