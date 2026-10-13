# Fisticuffs user

Bare-handed combat for the player. v1 is two jabs: left trigger is a left jab,
right trigger is a right jab. A small [`AttackState`](src/attack.rs) sits between
input and both gameplay events and the `Jab` clip so a combo graph can grow here
later without rewriting the mailbox path.

## Controls

| Input | Intent | Attack |
|---|---|---|
| Gamepad **LT** | `AttackLeft` (+ `Focus`) | Left jab |
| Gamepad **RT** | `AttackRight` (+ `UseItem`) | Right jab |
| Keyboard **F** / mouse **RMB** | left trigger | Left jab |
| Keyboard **G** / mouse **LMB** | right trigger | Right jab |

Jabs only start when the player is unarmed (`PlayerUse` absent). Held guns and
grenades keep LT/RT as focus/fire. Drop or holster the item to punch.

## Timing

The authored [`Jab`](../../world/layers/mobs/characters/animations/src/animations/jab.rs)
clip is one-shot at [`JAB_CYCLE_SPEED`](../../world/layers/mobs/characters/motion/src/clip.rs)
(`0.9`, **1.111 s** per punch). Phases are clip-progress windows:

| Phase | Clip progress | Seconds | What |
|---|---|---|---|
| **Startup** | `0.00 .. 0.42` | 0.467 s | Chamber (`BACKSWING_END = 0.18`) then extend |
| **Active** | `0.42 .. 0.52` | 0.111 s | Full-extension hold (hit window) |
| **Recovery** | `0.52 .. 1.00` | 0.533 s | Return to guard |

A press during startup or active **queues one** follow-up (latest wins). A press
during recovery **cancels into** the next jab immediately. That queue/cancel
slot is the combo-graph hook.

## Gameplay / visuals

`AttackState` owns time and side. During the active window the crate emits
[`FisticuffsStrike`](src/lib.rs) (`attacker`, `side`, `kind`, clip `progress`).
Damage resolution is a TODO: there is no fist sensor, and
[`HitCapsule`](../../player/src/hit.rs) is receive-only. Do not drive `Hit`
until a punch volume exists.

The visual layer writes `AnimClip::Jab` + `AnimProgress` on the body mailbox
**after** locomotion and **before** anim apply. It does not touch the capsule
motor, `MoveWish`, or hit volumes. Locomotion physics keep working; the walk
cycle is replaced for the punch because `Jab` is a **full-body** clip (lead-leg
stance, trunk, both arms). A later combo graph can switch to an upper-body mask
if gait should continue.

Mailbox already crossfades on `AnimId` change (walk ↔ jab) from the visible
pose. Same-`AnimId` restarts (left ↔ right, or recovery cancel) call
`AnimMailbox::restart_from_visible`, the runtime equivalent of
`Transition::from_visible`.

## Play

In the world / game, drop the held item, then tap LT/RT (or F/G, RMB/LMB).

The authored clip can still be scrubbed in `characters-playground`:

```bash
cargo run -p characters-playground -- character assemble --animation jab --side left
cargo run -p characters-playground -- character playback --progress 0.47
```
