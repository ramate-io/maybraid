# Character motion

Per-frame articulation for Crozon characters. Recipes in `crozon-characters`
**stamp** host identity; this crate **syncs** host motion markers from the shown
LOD band and **realizes** clips / pitch. Motion does not implement `LodScene` or
species recipes.

## Crate graph

```
ground                    # ElevationProbe, GroundHit
ground-avian              # AvianElevationProbe (SpatialQuery)

malo-animations           # Animation { apply_for, effects_for, apply }
crozon-rigs

crozon-character-motion   # this crate
        ↑
crozon-characters         # recipes; host() stamps initial markers
        ↑
playgrounds
```

## Dataflow (one frame, High band)

```
WASD / Space
  → physics capsule (Avian)
  → drive_player_locomotion writes AnimRefRoot on the body host

sync_motion_markers
  → shown LodLevelRoot (else desired / High) → motion_policy
  → insert/remove AnimateBones / AnimateEffects on body host
  → insert/remove ApplyTerrainPitch on character root

tick_anim_mailbox          # every body: advance clip time
apply_anim_mailbox         # With<AnimateBones|AnimateEffects>: sample + write
apply_terrain_pitch        # With<ApplyTerrainPitch>: Avian rays → visual rotation
```

UltraLow: sync strips markers → far hosts still tick time, but bone writes and
rays are archetype-filtered out.

## LOD: host markers

**Do not stamp motion markers on level-content children.** Chunk fulfill only
spawns nested rig/part hosts. Runtime truth is on the host:

| Where | What |
|---|---|
| `RigNode::host` (body) | `AnimRefRoot`; initial `AnimateBones` / `AnimateEffects` from `motion_policy` |
| Character spawn | Initial `ApplyTerrainPitch` from `motion_policy` |
| `sync_motion_markers` | Keeps those host markers aligned with the **shown** band |

| Level | bones | effects | pitch |
|---|---|---|---|
| High | yes | yes | yes |
| Medium | no | yes | yes |
| Low | no | yes | no |
| UltraLow / distance / resolution | no | no | no |

This is the default linear ramp in [`motion_policy`](src/policy.rs), not a
per-recipe regime. Sync a different map yourself to differ.

## Bevy systems

### In `crozon-characters` (`CharacterComponentsPlugin`)

Structural realize only. Sets: `Membership → InvalidateRefs → BoneMap → Fulfill → Pose`.

No clip sampling. No rays.

### In this crate (`CharacterMotionPlugin`)

| System | Set | Does |
|---|---|---|
| `sync_motion_markers` | `Anim` | Shown band → host marker insert/remove |
| `prepare_anim_mailbox` | `Anim` | Typed rig + `AnimMailbox` once the bone map is ready |
| `tick_anim_mailbox` | `Anim` | Advance time on every body mailbox |
| `apply_anim_mailbox` | `Anim` | Sample/write only `With<AnimateBones\|AnimateEffects>` |
| `apply_terrain_pitch<P>` | `Elevation` | **Not registered here** — app adds with a concrete probe; filters `With<ApplyTerrainPitch>` |

Order `CharacterMotionSystems::Anim` after `CharacterHostSystems::Pose`.
Order elevation after physics / locomotion.

### In playgrounds

| System | Does |
|---|---|
| Player physics / `ShapeCaster` | Capsule ground, jump |
| `drive_player_locomotion` | Wish → `AnimRefRoot` clip |
| `prepare_character_terrain_pitch` | Measure girdles, insert `TerrainPitch` |
| `sync_suspend_terrain_pitch` | `Jumping` → `SuspendTerrainPitch` on the body |
| `apply_terrain_pitch::<AvianElevationProbe>` | Probe colliders, write visual rotation |

## How to implement behaviors

### Play a clip

Insert `AnimRefRoot(AnimRef::new(AnimClip::walk()))` on the **body** rig host.
The mailbox transitions on `AnimId` (variant), not knob values.

### Gate work by LOD

Host markers + `With<>` on expensive systems. [`motion_policy`](src/policy.rs) is
the shared table `sync_motion_markers` applies. To add a capability, extend
`MotionPolicy` and sync a new host marker.

### Stand on colliders (not a heightfield)

Implement `ground::ElevationProbe` (or use `AvianElevationProbe`). Wrap the
generic apply loop in a concrete system. Side rays run only when
`TerrainPitch.roll_weight > 0`.

### Jump / airborne

Insert `SuspendTerrainPitch` on the **physics parent** of the visual.

## What this crate is not

- Species recipes or `impl LodScene`
- Socket / skin / proportion pose (those stay in `crozon-characters`)
- “Which hosts overlap this AABB?” (`lod::LodSceneRegionIndex`)
