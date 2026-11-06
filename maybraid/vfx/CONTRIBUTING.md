# Contributing to Maybraid VFX

Runtime crate [`maybraid-vfx`](lib/) plus the isolated [`vfx-playground`](playground/).
Domain shaders and world materials stay in their crates; this tree is the
effect graph — concepts land here before they are wired into firearms or world.

Repo-wide Rust rules (never `mod.rs`, methods on types, `crate::` paths) live in
the root [CONTRIBUTING.md](../../CONTRIBUTING.md). This file is the VFX layout.

## Authored assets vs runtime

Pack newly authored assets from local exports before committing the runtime
results. The packaging workflow is documented in
[`bin/vfx/README.md`](../../bin/vfx/README.md): no initial GitHub push or previous
commit is required. Runtime files live in [`maybraid/assets/vfx/dust`](../assets/vfx/dust);
large authoring projects and offline recordings are archived separately, never
fetched by an ordinary build.

The Rust crate loads those committed runtime files through Bevy's AssetServer
(`.flipbook.json`, `.volume.json`, KTX2). Do not decode source PNGs, repack
texture arrays, or fetch Houdini projects at startup.

## Effect graph

Authored VFX is a three-level stack. Lower a named recipe into layers; each
layer owns one part.

```text
effect (named composite / recipe)
  └─ layer (timed piece, delay + local pose + scale)
       └─ part (spawned primitive)
```

| Module | Owns | Examples |
| --- | --- | --- |
| [`lib/src/effects.rs`](lib/src/effects.rs) | Named recipes that implement `EffectRecipe` | `Dust`, `FieryExplosion` |
| [`lib/src/layers.rs`](lib/src/layers.rs) | Standalone visual layers that can spawn alone | `DustLayer`, flash, fireball, smoke, sparks |
| [`lib/src/parts.rs`](lib/src/parts.rs) | Spawned primitives and their assets / renderers | flipbook card, packaged KTX, mesh lobe, Hanabi particle |
| [`lib/src/composition.rs`](lib/src/composition.rs) | Graph schema | `EffectDefinition`, `EffectLayer`, `EffectPart`, `EffectRecipe` |

Spawn typed recipes with `commands.spawn_scene(Effect { recipe, spawn })`, or
the flattened `EffectDefinition` path through `VfxLibrary` and `spawn_vfx`.
Recipes stay on the root for typed queries (`With<Dust>`). Runtime clocks live
on `VfxInstance` and the part entities, not on the recipe.

### Parts

A part is something a layer actually spawns. Keep the graph types
(`FlipbookPart`, `MeshPart`, `ParticlePart`, `LightPulse`) in `composition.rs`
so `EffectPart` stays one enum. Put spawn, tick, materials, and loaders next to
the primitive:

| `parts/` module | Graph type | Role |
| --- | --- | --- |
| `flipbook_card` | `FlipbookPart` | Capture-plane renderer for packaged KTX |
| `packaged` | assets for `FlipbookPart` | `.flipbook.json` / `.volume.json` loaders and manifests |
| `lobe` | `MeshPart` | Overlapping mesh volumes and `LobeMaterial` |
| `particle` | `ParticlePart` | Hanabi instance properties, procedural sprite atlases, and spawn |

`particle/flipbook.rs` is Hanabi `SPRITE_INDEX` over lifetime for wisp cards.
That pipeline is not packaged KTX. Never merge it with `flipbook_card`.

Do **not** put these in `parts/`:

- `LightPulse` — a short `PointLight` fade, dispatched from `spawn.rs`.
- Host types (`VfxSpawn`, `VfxInstance`, membership, lifecycle clocks).

New primitive kinds join `parts/` as a sibling `foo.rs` (and `foo/` for
children). Do not grow `lib.rs` with another crate-root renderer.

### Host and shared utilities

| Module | Role |
| --- | --- |
| `spawn.rs` | `VfxSpawn`, `Effect<T>: Scene`, `realize_layer` dispatch |
| `lifecycle.rs` | Emitter gating, instance clocks, cleanup |
| `membership.rs` | `VfxMemberOf` vs `ChildOf` (lobes are grandchildren) |
| `library.rs` | Compiled handles and named definitions |
| `palette.rs`, `seed.rs`, `names.rs` | Shared color, seed streams, canonical names |

`VfxPlugin` registers packaged loaders, the card and lobe materials, Hanabi,
and lifecycle. Shared Hanabi assets are never mutated per instance.

## Module layout

Sibling `foo.rs` next to `foo/`. Never `mod.rs`. Same rule as the rest of the
repository:

```text
lib/src/parts.rs
lib/src/parts/flipbook_card.rs
lib/src/parts/flipbook_card/flipbook_card.wgsl
lib/src/parts/lobe.rs
lib/src/parts/lobe/material.rs
lib/src/parts/packaged.rs
lib/src/parts/packaged/manifest.rs
lib/src/parts/particle.rs
lib/src/parts/particle/assets.rs
lib/src/parts/particle/atlas.rs
lib/src/parts/particle/flipbook.rs
```

Bevy `embedded_asset!` paths follow `file!()`. After a move, update
`ShaderRef` to the path under `src/` (for example
`embedded://maybraid_vfx/parts/flipbook_card/flipbook_card.wgsl`).

Prefer methods on the part type (`FlipbookPart::spawn`, `MeshPart::spawn`) over
free functions in `spawn.rs`. Use `use crate::…` rather than `super::` chains.
Keep crate-root `pub use` of the types callers already import (`DustMaterial`,
`PackagedVfxPlugin`, `LobeMaterial`).

## Playground

[`vfx-playground`](playground/) is an isolated host for the graph. Preview a
recipe or a single layer there; do not add game, weapon, or world systems.
Commands and camera notes live in the [playground README](playground/README.md).

## Verify

```text
cargo test -p maybraid-vfx
cargo test -p vfx-playground
cargo run -p vfx-playground -- show dust
```
