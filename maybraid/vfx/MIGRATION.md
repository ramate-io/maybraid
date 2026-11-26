# VFX migration from main

Baseline: `main` at `57a8a30e` (the API before this PR).

## 1. Put placement on the entity

Before:

```rust
let dust = Dust::rolling_wave(&server, &mut meshes);
commands.spawn_scene(Effect {
    recipe: dust,
    spawn: VfxSpawn {
        transform: Transform::from_xyz(2.0, 0.0, 4.0),
        scale: 0.8,
        seed: Some(7),
        ..default()
    },
});
```

After (using `bevy::scene::prelude::bsn`):

```rust
let placement = Transform::from_xyz(2.0, 0.0, 4.0)
    .with_scale(Vec3::splat(0.8));
commands.spawn_scene(bsn! {
    ~{placement}
    @{Effect::new(Dust::rolling_wave())
        .with_spawn(VfxSpawn { seed: Some(7), ..default() })}
});
```

`VfxSpawn` keeps intensity, playback, tint, seed, and palette. Its `transform`,
`scale`, and scale-clamping APIs are removed, along with `ScaleBounds` and
`EffectDefinition::with_scale_bounds`. If both old placement scale and scalar scale
were set, multiply them into the entity's `Transform::scale`. Apply any desired
scale limits at your call site. `spawn_vfx` still returns a root entity; insert its
`Transform` to place it. Existing layer transforms remain relative to that root.

## 2. Author flipbooks by path; compose reusable layers

Replace `FlipbookPart::load(&server, path, mesh, duration)` with
`FlipbookPart::new(path)`, or use the shorter layer constructor below. Loading,
the shared proxy mesh, and manifest timing are supplied by the runtime.

```rust
use bevy::prelude::*;
use maybraid_vfx::{EffectLayer, EffectLayerGroup, EffectRecipe};

fn dust_cloud() -> EffectLayerGroup {
    EffectLayerGroup::new([
        EffectLayer::flipbook_path("vfx/dust/rolling_wave.flipbook.json")
            .with_name("ground dust"),
        EffectLayer::flipbook_path("vfx/dust/rolling_wave.flipbook.json")
            .with_name("echo")
            .with_translation(Vec3::X * 0.5)
            .with_delay(0.15),
    ])
}

#[derive(Component, Clone, EffectRecipe)]
#[effect(name = "impact", layers = Self::layers)]
struct Impact { size: f32 }

impl Impact {
    fn layers(&self) -> EffectLayerGroup {
        dust_cloud().scaled(self.size).delayed(0.05)
    }
}
```

The derive calls your layer function with `&self`; handwritten `EffectRecipe`
implementations still work. Arrays, vectors, and groups are accepted. Prefer layer
constructors; direct `EffectLayer` literals now also need `name: None` (or a label). Combine
multiple groups with `EffectLayerGroup::from_groups`, or use
`EffectDefinition::from_groups` in a handwritten recipe.

`with_translation` / `with_rotation` preserve the other transform fields.
Layer `with_scale` multiplies its local axis scale without moving it. Group
`scaled` also scales offsets; `rotated`, `translated`, and `delayed` compose
in call order. No new effect needs its own runtime system or asset registry.

## 3. Preview and iterate

In your playground app, register a factory once:

```rust
use vfx_playground::VfxPreviewAppExt;
app.register_vfx_preview("impact", || Impact { size: 0.8 });
```

Run `/show impact --seed 7`. The inspector shows zero-based layer indices,
pending/loading/playing/finished state, and flipbook frames. `/solo 1`, `/mute 0`,
`/unsolo`, and `/unmute` rebuild the selection with the same seed. `/pause` then
`/step 0.0666667` advances simulation by that many seconds; playback speed still
applies. `/reset` rebuilds from the factory. `/show` clears layer selections.

For asset edits, `/reload` requests manifest/image reloads; `/reset` after loading
recreates materials. Rust recipe edits require recompilation. Failures show the
recipe, layer, manifest path, and loader/dependency error, and stop automatic replay.
