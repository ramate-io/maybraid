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
        │             EyeGlow / SpatialRumble / Impact / ParadeExplosion cues
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

```shell
cargo test -p bikanwarda-attacks --all-targets
cargo run -p bikanwarda-attacks-playground
```

Headless capture (Bevy `Screenshot` readback; X11 `scrot` of lavapipe stays black):

```shell
cargo build -p bikanwarda-attacks-playground
xvfb-run -a -s "-screen 0 1280x720x24" maybraid/attacks/bikanwarda/preview/capture.sh
```
