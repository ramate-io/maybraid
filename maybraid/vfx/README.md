# Dust and smoke VFX — usage specification

Reusable Bevy effects for landing dust, rolling dust waves, running herds and smoke explosions. Cartoon uses opaque shaded surfaces; Combined uses filled density with fine smoke and grains. Styles are selected explicitly and never substituted automatically.

The effects are implemented in `maybraid-vfx`. The adjacent playground demonstrates them; gameplay events, animal movement and terrain acquisition remain the host game's responsibility.

## Requirements

- Use this workspace's pinned Rust toolchain and Bevy dependencies.
- Add `VfxPlugin` once. The Maybraid world already includes it.
- Set the asset root to `maybraid/assets` when launching from the repository root. Runtime inputs are packaged under `assets/vfx`; Houdini is not required.
- Add `bevy::core_pipeline::prepass::DepthPrepass` to active cameras displaying Combined. The legacy baked dust also uses it for correct intersections.
- Load and warm effects before time-critical gameplay. Dependency loading delays playback, but first-use GPU pipeline compilation can still affect the first presentation.

## Choose a category

| Category | Effects | Game-side input |
|---|---|---|
| [One-shot effects](#one-shot-effects) | Landing, RollingWave and smoke explosion, each in Cartoon / Combined | Spawn once when an event occurs |
| [Contact effects](#contact-effects) | Landing and RollingWave, each in Cartoon / Combined | Spawn once, then submit nearby collider snapshots |
| [Herd effects](#herd-effects) | Trail and Cloud, each in Cartoon / Combined | Create one herd emitter, then continuously submit movement and terrain |

Contact effects also have finite lifetimes; they are separated here because their interaction input differs from ordinary one-shot playback.

Load recipes with an `&AssetServer`, retain them in a game resource, and clone them when spawning. `VfxLibrary::get` also provides named definitions for one-shot effects. `combined-dust` remains an alias for Combined wave. The older `dust` / `rolling-wave` names select the baked multi-view effect, not the 3D wave.

## One-shot effects

| Effect | Typed recipe | Playground name |
|---|---|---|
| Cartoon landing | `effects::cartoon_landing::CartoonLanding::load` | `cartoon-landing` |
| Combined landing | `CombinedLanding::load` | `combined-landing` |
| Cartoon rolling wave | `effects::cartoon_wave::CartoonWave::load` | `cartoon-wave` |
| Combined rolling wave | `CombinedWave::load` | `combined-wave` |
| Cartoon smoke explosion | `CartoonExplosion::load` | `cartoon-explosion` |
| Combined smoke explosion | `CombinedExplosion::load` | `combined-explosion` |

The existing library also retains baked `Dust`, `FieryExplosion` and the individual flash/fireball/smoke/sparks layers. `FieryExplosion` is distinct from the new smoke-only explosions.

### Landing example

Inside an existing gameplay system, spawn a cached recipe through the common API:

```rust,ignore
use bevy::prelude::*;
use maybraid_vfx::{CombinedLanding, Effect, VfxSpawn};

let recipe = CombinedLanding::load(&server); // Cache this during setup.
commands.spawn_scene(Effect {
    recipe,
    spawn: VfxSpawn {
        transform: Transform::from_translation(ground_contact_position),
        seed: Some(13),
        ..default()
    },
});
```

Trigger landing once on the airborne-to-grounded transition. Use the physical ground contact point rather than the character mesh centre. Keep the VFX root independent of the character. The runtime owns loading, playback and completion; removing the root cancels the effect.

`VfxSpawn.playback` is clamped to 0.25–4. Instance age already includes playback. Do not multiply it again. Positive uniform visual scale uses the host's 0.25–4 range where the selected effect supports scaling; contact effects require scale 1.

Original landing main motion lasts about 1.05 s; Cartoon completes at 1.2 s and Combined at 2.4 s including secondaries. Cartoon wave completes at 4 s; Combined wave at 4.7 s. These are one-shot events, not seamless loops. Original Combined landing assumes the world Y=0 ground plane; use its contact recipe for an elevated horizontal floor.

### Smoke explosion example


```rust,ignore
use bevy::prelude::*;
use maybraid_vfx::{CombinedExplosion, Effect, VfxSpawn};

let mut recipe = CombinedExplosion::load(&server); // Cache during setup.
recipe.part.heading = Some(Vec3::X); // Explicit world heading; no seeded yaw.
commands.spawn_scene(Effect {
    recipe,
    spawn: VfxSpawn {
        transform: Transform::from_translation(explosion_position),
        scale: 1.0,
        playback: 1.0,
        seed: Some(6),
        ..default()
    },
});
```

For Cartoon, construct `CartoonExplosion::load(&server)` and use the same `Effect` / `VfxSpawn` structure. Neither explosion requires `ContactInputs` or a continuous input queue. Combined still requires camera `DepthPrepass`.

Both recipes share one baked 14 s motion track, including subdivision. Runtime seed changes yaw, not the authored motion or grain births. These are smoke-only effects; the existing `FieryExplosion` remains separate.

Translation, upright yaw and positive uniform scale are supported. Set `recipe.part.heading` (horizontal world vector) to determine the final yaw; otherwise spawn yaw and seeded yaw compose. The light direction remains world-fixed. Generic explosion palette/intensity settings are unused; tint multiplies linear colour.

Cartoon exposes `ExplosionLod::{Full,Medium,Low}`. Combined exposes `ExplosionQuality::{High,Medium,Low}`. Quality never changes style. Explosions have depth occlusion but no actor avoidance.

## Contact effects

| Effect | Typed recipe | Playground name |
|---|---|---|
| Cartoon landing contact | `CartoonContact::landing` | `cartoon-landing-contact` |
| Combined landing contact | `CombinedLanding::contact` | `combined-landing-contact` |
| Cartoon wave contact | `CartoonContact::wave` | `cartoon-wave-contact` |
| Combined wave contact | `CombinedWaveContact::load` | `combined-wave-contact` |

Contact recipes support world translation, elevation and yaw, with scale 1 and a horizontal floor at spawn Y. Pitch, roll, shear and root movement after admission are unsupported; moving an admitted root cancels it. Contact effects do not conform to uneven terrain.

Attach `ContactInputs` when spawning so the effect uses game data rather than the playground's default scripted interaction:

```rust,ignore
use maybraid_vfx::{CombinedLanding, ContactInputs, Effect, VfxSpawn};

let root = commands.spawn_scene(Effect {
    recipe: CombinedLanding::contact(&server),
    spawn: VfxSpawn {
        transform: Transform::from_xyz(20., 8., -15.)
            .with_rotation(Quat::from_rotation_y(37_f32.to_radians())),
        seed: Some(13),
        ..default()
    },
}).insert(ContactInputs::default()).id();
```

Query the root's `VfxInstance` and `ContactInputs` after admission. Submit nearby colliders in world coordinates:

```rust,ignore
use bevy::math::DVec3;
use maybraid_vfx::WorldContactSnapshot;

inputs.submit(WorldContactSnapshot::from_world_rates(
    instance.age as f64,
    instance.playback as f64,
    nearby_colliders,
    DVec3::ZERO, // World wind, metres per game second.
    teleported,
)?)?;
```

Colliders are `parts::combined::interaction::Interactor`: stable ID, measured capsule/box dimensions, world position/orientation, linear/angular velocity and interaction settings. Put the primary actor first: its analytic compression/wake response is primary; all selected solids participate in the field and exclusion.

The helper converts game-second velocities to effect-age rates once. Direct snapshots expect rates already converted. Equal timestamps replace; backward timestamps branch history. Teleports are discontinuities. Gaps above 0.25 s are not interpolated; missing future data holds solids still. The 256-snapshot history is bounded; resubmit history for older seeks. Submit an empty collider list to remove solids.

Each region is a fixed local 6×3×6 m field. Supply nearby actors, at most 20 unique colliders per snapshot; invalid submissions return `Err`. `ContactRegionBudget.maximum` defaults to 2. Overflow rejects new effects without replacing active ones. Combined contact completes at 2.4 s for landing / 5.7 s for wave, including secondary tails.

## Herd effects

All four selections use `effects::herd::HerdDust::load(&server, settings)` with `settings.travelling=true`:

| Effect | `Settings.group` | `Settings.style` | Playground arguments |
|---|---|---|---|
| Cartoon Trail | `false` | `Style::Cartoon` | `herd-dust --mode trail --style cartoon` |
| Combined Trail | `false` | `Style::Combined` | `herd-dust --mode trail --style combined` |
| Cartoon Cloud | `true` | `Style::Cartoon` | `herd-dust --mode cloud --style cartoon` |
| Combined Cloud | `true` | `Style::Combined` | `herd-dust --mode cloud --style combined` |

One herd root owns an emitter group and released dust. Keep the root unparented, at identity; move the actor inputs instead. `group=false` selects Trail; `group=true` selects Cloud, covering the herd's rear portion and wake.

```rust,ignore
use std::sync::Arc;
use maybraid_vfx::{Effect, VfxSpawn};
use maybraid_vfx::effects::herd::HerdDust;
use maybraid_vfx::parts::herd::{Inputs, Settings, Style};
use maybraid_vfx::parts::herd::terrain::{Heightfield, HerdTerrain};

let terrain = Heightfield::new(
    origin_xz, node_spacing, dimensions, version, heights, validity,
)?;
let recipe = HerdDust::load(&server, Settings {
    travelling: true,
    group: true,
    style: Style::Combined,
    ..default()
});
let root = commands.spawn_scene(Effect {
    recipe,
    spawn: VfxSpawn { seed: Some(13), ..default() },
}).insert((Inputs::default(), HerdTerrain(Arc::new(terrain)))).id();
```

Supply `parts::herd::Frame` through `Inputs::push` before `HerdTick`:

| Field | Host supplies |
|---|---|
| `time` | Consecutive 120 Hz effect-age timestamps; first sample is 1/120 s |
| `emitters` | Stable IDs, world ground-contact positions, velocity, ground normal/grounded flag and discontinuity |
| `colliders` | Actual world capsule/box geometry, orientation and velocities |
| `heights` | Optional measured body/shoulder heights, keyed by actor ID |
| `births` | Empty for ordinary automatic distance/formation emission |
| `style` | Cartoon or Combined for new births; existing puffs retain their style |
| `emitting` | Whether to allow new births |

Wait until the root is armed. Resample game movement into consecutive effect-age samples; do not interpolate teleports. Up to 512 samples may be queued. `Frame::from_world_rates(playback)` converts game-second velocities once without changing timestamps or positions. Do not apply it to rates already measured in effect-age time.

Emission is driven by travelled distance, not foot animation. An in-place running animation needs actual travel input to emit. Trail and Cloud use measured actor dimensions; moving or turning the herd never transforms released dust. The world-aligned field shifts by whole cells and preserves overlapping velocity state. Outside solved coverage, dust continues ambient motion, drag, settling and expiry.

### Terrain

`Heightfield::new` takes world-XZ origin, positive node spacing, `[nx,nz]`, version, world heights and validity. Each axis has 2–65 nodes; indexing is `x + nx*z`. Sampling is bilinear and normals are derived from heights.

Replace `HerdTerrain` with a new immutable snapshot when terrain or coverage changes. Cover `HerdState::terrain_requirements()`: the active field plus all live released supports, not just the current actors. Missing terrain rejects births; old dust has a counted last-contact-height fallback that cannot reconstruct unknown ledges.

Static single-valued terrain, slopes and modest height changes are supported. Caves, overhangs, moving platforms and arbitrary collision meshes are not. Gentle main-body settling defaults to 0.12 m/s² acceleration and 0.12 m/s terminal speed, in effect-age units. Grains retain stronger gravity.

Set `emitting=false` to let old dust drain; keep submitting the required samples while draining. Despawn the root to cancel. For replay, create a fresh root and replay the same seed and inputs.

## Budgets and limits

| System | Default / allocation |
|---|---|
| Contact | 2 independent fixed regions; 20 colliders per input |
| Travelling Herd | 2 herds; up to 20 emitters/colliders per snapshot |
| Herd pools | 32 main bodies, 64 clouds, 64 grains per herd |
| Herd flow | One rolling 6×3×6 m field per herd; emitting footprint must fit |
| Combined Herd optics | Up to 4 live birth tiles per herd; about 22.5 MiB texture storage per tile plus bounded buffers |
| Smoke explosions | Up to 8 Cartoon / 3 Combined; Combined reserves 160 MiB field textures per instance |

Full pools reject new admissions without evicting live effects. Limits are allocation guards, not frame-rate guarantees. Cartoon contact/terrain surface reconstruction is CPU-intensive. Large Combined populations also exceed a 60 FPS budget in measured Mac scenes; profile the actual game workload before raising limits.

Separate optical regions use approximate back-to-front composition, not joint extinction or cross-region fluid simulation. Combined interiors can look uniform at high density. Very large world coordinates require host floating-origin handling. Contact floors remain horizontal even though travelling Herd supports sampled slopes.

## Preview and source layout

See [playground commands](playground/README.md) for interactive examples. Runtime code is in `lib`, assets in `../assets/vfx`, and packaging instructions in `../../bin/vfx/README.md`. The effect graph and extension rules are in [CONTRIBUTING.md](CONTRIBUTING.md).

No game event handlers, animal AI or terrain engine dependency is installed by this library. Connect the interfaces above to the host's existing landing, movement and collision systems.
