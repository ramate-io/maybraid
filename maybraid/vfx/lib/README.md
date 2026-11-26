# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, mesh lobes, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. First delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `fiery_explosion`. Module layout and the authored-asset / runtime split are in [CONTRIBUTING.md](../CONTRIBUTING.md).

Near fire and smoke use overlapping mesh lobes with a crate-owned stylized material. Flipbook cards remain the distant / wisp layer. Spawn typed recipes with `Effect::new(recipe)`, or use the flattened `Vfx` / `spawn_vfx` path. Root placement is ordinary scene composition; layer-local transforms and scales remain relative. Intensity scales particle count and emission. Playback speeds the authored layer envelope without changing those ratios. `ExplosionPalette` recolors mesh, particles, and light; `tint` multiplies that palette. Flipbooks store heat and shape, not baked orange. An omitted seed is generated; an explicit seed reproduces sampled lobe variation. Shared Hanabi assets are never mutated per instance.

```text
cargo test -p maybraid-vfx
cargo run -p vfx-playground
```

## Authoring and placement

Recipes do not load flipbook assets or allocate proxy meshes:

```rust
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use maybraid_vfx::{Dust, Effect, EffectDefinition, EffectLayer, EffectRecipe, FlipbookPart, FlipbookRef, VfxSpawn};

#[derive(Component, Clone)]
struct RollingDust;

impl EffectRecipe for RollingDust {
    fn effect_definition(&self) -> EffectDefinition {
        EffectDefinition::new("rolling-dust", [
            EffectLayer::flipbook(FlipbookPart::new(
                FlipbookRef::path(Dust::ROLLING_WAVE),
            ))
            .with_delay(0.15)
            .with_transform(Transform::from_xyz(0.0, 0.1, 0.0))
            .with_scale(0.8),
        ])
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Transform::from_xyz(2.0, 0.0, 4.0).with_scale(Vec3::splat(1.5))
        @{Effect::new(RollingDust).with_spawn(VfxSpawn {
            playback: 0.5,
            seed: Some(7),
            ..default()
        })}
    });
    // Existing baked recipe, at identity by default:
    commands.spawn_scene(Effect::new(Dust::rolling_wave()));
}
```

Any direct `.flipbook.json` path works without a central registry entry.
`FlipbookRef` follows `SceneRef`'s path identity convention: Bevy's `AssetServer`
deduplicates the typed asset and owns dependency loading. The shared runtime
resolves a reference when its layer is realized, supplies one standard rectangle
proxy mesh per world, and stores the handle separately on the runtime child.
Recipes and pending layers contain no resolved flipbook handles.

`VfxSpawn` contains playback, seed, tint, intensity, and palette only. The effect
requires an identity `Transform` if absent; it never inserts a competing placement
or clamps the caller's scale. For the flattened helper, insert a `Transform` on
the entity returned by `spawn_vfx`. Layer `transform.scale` and `layer.scale`
multiply within the root hierarchy.

Global-space Hanabi already transforms initial positions and velocities, so these
are not scaled again in instance properties. Billboard sizes, update acceleration,
and point-light range use the propagated world scale once. Their scalar scale uses
the largest absolute axis for nonuniform placement; geometry retains the full
transform. Soundwave world-space uniforms also follow the propagated layer pose.

## Packaged dust

`VfxLibrary::dust` is the shared eight-view baked dust recipe. It lowers through
the same layer composition, spatial scale, playback controls, instance ownership,
and cancellation/cleanup as other effects. `commands.spawn_scene(Effect::new(recipe))`
retains `Dust` on the root for typed queries. Each instance owns its material
clock; all instances share the KTX2 texture and standard proxy mesh. `intensity` adjusts this baked
layer's brightness rather than spawning extra particles.

`PackagedVfxPlugin` registers typed `PackagedFlipbook` and `StaticVolume` loaders
for `.flipbook.json` and `.volume.json`. `VfxPlugin` includes it automatically.
Images load through Bevy's standard KTX2 decoder with linear clamp sampling;
their manifest-relative paths and handles remain normal AssetServer
dependencies. Configure the application's asset root to `maybraid/assets`, as
the game and playground already do. Await recursive dependency loading before
using a manifest's image. The runtime waits for the manifest and all image dependencies before starting at
frame zero, and derives duration from manifest timestamps. `EffectDefinition::duration()`
is only a planned lower bound for unresolved flipbooks; completion uses actual
layer clocks. The playground repeats after the previous instance finishes.
A manifest or dependency failure cancels the entire owning instance. Despawning
the root cancels loading/playback children and pending layers together; runtime
handles and per-layer materials are released through Bevy's normal asset lifecycle.

Use a camera with Bevy's `DepthPrepass` for independent scene-depth rejection
and soft contact fading of each captured plane; the playground enables it.
Without that prepass, the material uses an approximate blended depth. The bake
was authored at 20° elevation and supplies no scene lighting response or
volumetric simulation. Cold GPU pipeline compilation may skip opening frames;
warm the effect before comparing a precise authored timestamp.

See [asset metadata and provenance](../../assets/vfx/dust/README.md),
[offline export tools](../../../bin/vfx/README.md), and
[playground controls](../playground/README.md).

## Authoring conveniences

The [migration guide](../MIGRATION.md) has before/after examples from `main`.
Use `EffectLayer::flipbook_path(path)` for a resource-free flipbook layer, optionally
labelled with `with_name`. `with_translation` and `with_rotation` preserve other
local transform fields. `EffectLayerGroup` collects reusable layers and applies
relative delay, translation, rotation, and uniform scale in call order, then flattens
into the existing runtime. Group scaling moves offsets as well as resizing parts;
layer `with_scale` only resizes the part.

The re-exported `EffectRecipe` derive accepts
`#[effect(name = "impact", layers = Self::layers)]`. The layer function receives
`&self` and returns an iterator of `EffectLayer` (including a group). Generic recipe
parameters and handwritten trait implementations remain supported. The derive adds
no systems or registration requirements. See the playground's `VfxPreviewAppExt`
for optional custom `/show` factories and layer inspection.
