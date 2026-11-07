# Bikanwarda attack GLB requirements

This crate ships **placeholders only**. Do not add a final Blender export here.
[#1117](https://github.com/ramate-io/maybraid/issues/1117) and the asset lanes
own the encounter body. Until those land, the playground uses primitive
stand-ins and the sockets below.

## Required sockets (names are illustrative)

| Socket | Component | Used by | Notes |
|---|---|---|---|
| `mouth` | `BikanwardaMouthSocket` | Bray `HazardOrigin::Emission` | Oral cavity. Pass the entity as `BeginAttack::emission_socket`. Local −Z should face the travel heading when the head is at rest. |
| `eye.L` / `eye.R` | `BikanwardaEyeSocket` | God Beam eye-glow cue | Two emissive points. A single `eyes` locator is enough for the placeholder. |
| `head` | `BikanwardaHeadSocket` | Bray head-flick placeholder | Neck / skull joint. The placeholder rotates local X during windup and restores the rest pose. |

Boss scale in [#1104](https://github.com/ramate-io/maybraid/issues/1104) is a
**20 m Epiphant**. Mouth height should sit on the head, not at the feet, so
sonic rings leave the mouth and travel horizontally at constant size
([#1147](https://github.com/ramate-io/maybraid/pull/1147) wave contract).

## Presentation that is not a GLB

| Cue | Host mapping |
|---|---|
| `ParadeExplosionCue` | Existing `fiery_explosion` (`maybraid-vfx`). Scale is a host choice; grenades use `blast_radius / 0.5`. |
| God Beam column | [#1150](https://github.com/ramate-io/maybraid/pull/1150) / [#1122](https://github.com/ramate-io/maybraid/issues/1122) follow `AttackPresentationEvent` + `HazardShape::Cylinder`. Width convention is still open. |
| Bray rings | [#1150](https://github.com/ramate-io/maybraid/pull/1150) / [#1123](https://github.com/ramate-io/maybraid/issues/1123) follow `WavefrontPresentation`. Do not simulate motion in VFX. |
| `SpatialRumbleCue` | Distance-attenuated pad rumble / camera shake. |
| `ImpactCue` | Separate landing / stomp channel. |

Face glow can later drive
[`face_material.rs`](../../world/layers/mobs/characters/shaders/src/face_material.rs)
instead of the placeholder spheres.
