# Bikanwarda attacks

Authored Parade, God Beam, and Bray recipes for
[#1119](https://github.com/ramate-io/maybraid/issues/1119). Gameplay is the
reusable [`attacks`](../README.md) crate
([#1147](https://github.com/ramate-io/maybraid/pull/1147)). This crate does not
fork that API and does not register into Self / `maybraid-world`.

```text
Encounter / summon
    BeginAttack(bikanwarda.parade | god_beam | bray)
        │
        ▼
attacks runtime          this crate
 windup → execute ──► Parade sequencer (emission blasts)
        │             presentation adapter → lane cues (always)
        │             optional placeholder meshes (debug)
        ▼
   damage::Hit            VFX / animation / rumble subscribers
```

Summoned Ward / Enraged forms ([#1106](https://github.com/ramate-io/maybraid/issues/1106))
invoke the same `AttackId`s. Victim policy is `BeginAttack::allies` plus the
authored `FriendlyFire::ExcludeOwnerAndAllies`. Copy a definition only if a
summon needs a different `FriendlyFire`.

## Agreed vs placeholder

| Fact | Source |
|---|---|
| God Beam active 10 s, 7 m column | [#1119](https://github.com/ramate-io/maybraid/issues/1119), [#1104](https://github.com/ramate-io/maybraid/issues/1104) |
| Column sits on `HazardOrigin::AimPoint` | [#1147](https://github.com/ramate-io/maybraid/pull/1147) |
| Bray rings leave the mouth at constant size, travel horizontally | [#1147](https://github.com/ramate-io/maybraid/pull/1147) |
| Parade: line toward the player, then sequential explosions | [#1119](https://github.com/ramate-io/maybraid/issues/1119) |
| Shared damage / cadence / cooldown / interruption / friendly fire | `AttackDefaults::AUTHORED` |

Everything else is [`BikanwardaAttackParams::PLACEHOLDER`](src/params.rs) and is
listed as an open question on the PR.

## Presentation adapter

[`presentation_adapter_plugin`](src/adapter.rs) maps [`AttackPresentationEvent`](../src/presentation.rs)
to stable cues. Sibling lanes wire these without this crate depending on them:

| Cue | Consumer |
| --- | --- |
| [`ParadeExplosionCue`](src/cues.rs) | `maybraid-vfx` `fiery_explosion` via [`consumers::parade_vfx`](src/consumers/parade_vfx.rs) |
| [`SpatialRumbleCue`](src/cues.rs) / [`ImpactCue`](src/cues.rs) | [`maybraid-spatial-rumble`](../../spatial-rumble/lib) via [`consumers::rumble`](src/consumers/rumble.rs) |
| [`BrayAnimationCue`](src/cues.rs) | `AnimClip::epiphant_bray` via [`consumers::bray_animation`](src/consumers/bray_animation.rs) ([#1141](https://github.com/ramate-io/maybraid/pull/1141)) |
| mouth / [`BikanwardaMouthSocket`](src/presentation.rs) | [`sample_bray_mouth_emit`](../../world/layers/mobs/characters/motion/src/mouth_emit.rs) via [`consumers::mouth_emit`](src/consumers/mouth_emit.rs) |
| [`GodBeamVfxCue`](src/cues.rs) / [`SonicWaveVfxCue`](src/cues.rs) | **PLACEHOLDER** meshes until `vfx-driven` + [#1150](https://github.com/ramate-io/maybraid/pull/1150) |
| [`EyeGlowCue`](src/cues.rs) | [`PendingEyeGlow`](src/consumers/eye_glow.rs) until `eye-glow` + [#1152](https://github.com/ramate-io/maybraid/pull/1152) |

Enable [`BikanwardaPresentationConsumersPlugin`](src/consumers/mod.rs) (on by default via `presentation-consumers` feature). Gameplay does not need combat HUD.

[`placeholder_presentation_plugin`](src/presentation.rs) (eye spheres, head flick) is
**opt-in** via `BikanwardaAttacksPlugin { placeholders: true }`.

```shell
cargo test -p bikanwarda-attacks --all-targets
cargo run -p bikanwarda-attacks-playground
```

Headless capture (Bevy `Screenshot` readback; X11 `scrot` of lavapipe stays black):

```shell
cargo build -p bikanwarda-attacks-playground
xvfb-run -a -s "-screen 0 1280x720x24" maybraid/attacks/bikanwarda/preview/capture.sh
```
