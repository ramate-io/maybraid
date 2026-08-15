# Character motion

Per-frame articulation for Crozon characters. Recipes in `crozon-characters`
**stamp** identity; this crate **realizes** it. Motion does not implement
`LodScene` or species recipes.

## Crate graph

```
ground                    # ElevationProbe, GroundHit
ground-avian              # AvianElevationProbe (SpatialQuery)

malo-animations           # Animation { apply_for, effects_for, apply }
crozon-rigs

crozon-character-motion   # this crate
        ↑
crozon-characters         # recipes; host() / scene_with_level stamp markers
        ↑
playgrounds
```

## Dataflow (one frame, High band)

```
WASD / Space
  → physics capsule (Avian)
  → drive_player_locomotion writes AnimRefRoot on the body host

shown LodLevelRoot has AnimateBones + AnimateEffects + ApplyTerrainPitch
  → tick_anim_mailbox
       apply_for  → bone pose (if AnimateBones)
       effects_for → armature root-motion (if AnimateEffects)
  → apply_terrain_pitch::<AvianElevationProbe>
       2× hit_down (front/hind); sides only if roll_weight > 0
       slerp visual rotation; capsule owns Y
```

UltraLow: same `AnimRefRoot` on the host, shown child has no markers → mailbox
advances time only; no bone writes, no rays.

## LOD: host vs shown child

**Do not rebuild a level to flip a bool.** Warm bands already have the right marker.

| Where | What |
|---|---|
| `RigNode::host` (body) | `AnimRefRoot`, `AnimateBones`, `AnimateEffects` (capability / fallback) |
| `RigNode::scene_with_level` | per-band `AnimateBones` / `AnimateEffects` |
| Character spawn / `scene_with_level` | `ApplyTerrainPitch` on High/Medium; omitted on Low/UltraLow |

Systems query the **shown** `LodLevelRoot` (and its content children). If no
level exists yet, they fall back to the host’s own markers.

| Level | bones | effects | pitch |
|---|---|---|---|
| High | yes | yes | yes |
| Medium | no | yes | yes |
| Low | no | yes | no |
| UltraLow / distance / resolution | no | no | no |

This is the default linear ramp in [`motion_policy`](src/policy.rs), not a
per-recipe regime. Stamp markers yourself in `scene_with_level` to differ.

## Bevy systems

### In `crozon-characters` (`CharacterComponentsPlugin`)

Structural realize only. Sets: `Membership → InvalidateRefs → BoneMap → Fulfill → Pose`.

No clip sampling. No rays.

### In this crate (`CharacterMotionPlugin`)

| System | Set | Does |
|---|---|---|
| `prepare_anim_mailbox` | `Anim` | Insert typed rig, `AnimBone`s, `AnimMailbox` once the bone map is ready |
| `tick_anim_mailbox` | `Anim` | Advance time; `apply_for` / `effects_for` gated by the shown child |
| `apply_terrain_pitch<P>` | `Elevation` | **Not registered here** — the app adds it with a concrete `ElevationProbe` |

Order `CharacterMotionSystems::Anim` after `CharacterHostSystems::Pose`.
Order elevation after physics / locomotion.

### In `ground-avian`

No character systems. Only `AvianElevationProbe`.

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

Insert `AnimRefRoot(AnimRef::new(AnimClip::walk()))` on the **body** rig host
(the entity with `AnimRefRoot` from `RigNode::host`). The mailbox transitions
on `AnimId` (variant), not knob values.

`ConceptAnimation` maps to `AnimClip` in `crozon-characters` (`From` impls).
Do not put concept types in this crate.

### Author a new clip

1. Implement `Animation<Rig>` in `malo-animations`:
   - `apply_for` writes bones only
   - `effects_for` is read-only (lengths + time) and returns `Effects`
   - `apply` is the default wrapper — do not override it
2. Composites (`Mix`, `Transition`) must call the split on children.
3. Add a variant to `AnimClip` / `AnimId` here and a `From<ConceptAnimation>`
   arm in `crozon-characters` if the concepts screen should list it.

### Gate work by LOD

Stamp markers in `scene_with_level`. [`motion_policy`](src/policy.rs) is the
shared linear default (High → UltraLow drops work). A different
`LodSceneLevel` → marker map is a different stamp in `scene_with_level`, not
a parameter to that function. To add a new capability, extend `MotionPolicy`
and the systems that read `shown_level_has::<YourMarker>`. Do not rebuild a
level to flip a bool.

### Stand on colliders (not a heightfield)

Implement `ground::ElevationProbe` (or use `AvianElevationProbe`). The apply
loop is a **function** generic over the probe — wrap it in a concrete Bevy
system that takes your `SystemParam` probe (generic `fn`s are not systems):

```rust
fn apply_avian_terrain_pitch(time: Res<Time>, probe: AvianElevationProbe, /* queries */) {
    apply_terrain_pitch(time, probe, /* queries */);
}

// then
apply_avian_terrain_pitch
    .in_set(CharacterMotionSystems::Elevation)
```

Exclude the physics body so the capsule is not a hit. Short max distance
(`PROBE_MAX_DISTANCE`). Side rays run only when `TerrainPitch.roll_weight > 0`
(family default is 0).

Opt a species into bank by setting `TerrainPitch.roll_weight` after prepare.
Keep sagittal pitch; do not write visual Y (capsule owns height).

### Jump / airborne

Insert `SuspendTerrainPitch` on the **physics parent** of the visual. Pitch
blends to 0 until it is removed.

## What this crate is not

- Species recipes or `impl LodScene`
- Socket / skin / proportion pose (those stay in `crozon-characters`)
- “Which hosts overlap this AABB?” (`lod::LodSceneRegionIndex`)
