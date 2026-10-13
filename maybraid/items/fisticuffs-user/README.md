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
| Keyboard **F** | left trigger | Left jab |
| Keyboard **G** | right trigger | Right jab |

Mouse **RMB** is analog focus / aim and **LMB** is UseItem. They share the
trigger axes but **do not** emit `AttackLeft` / `AttackRight` — starting to aim
must not throw a jab.

Jabs only start when the player is unarmed (`PlayerUse` absent). Held guns and
grenades keep LT/RT as focus/fire. Drop or holster the item to punch.

## Aim

[`PlayerLook`](../../player/src/identity.rs) is camera **−Z** forward. The `Jab`
clip is character space (**+Z** fight-forward, **+X** right). [`JabAim`](src/aim.rs)
converts look into that frame, clamps to a punching cone, and quantizes:

| Axis | Clamp | Step | Buckets |
|---|---|---|---|
| Pitch | −24° (belly) … +36° (above head) | 12° | −24, −12, 0, +12, +24, +36 |
| Yaw | ±24° off body facing | 12° | −24, −12, 0, +12, +24 |

Level look maps to `DEFAULT_JAB_TARGET`. The same quantized character-space
point is written to `JabParams.target` and [`FisticuffsStrike::target`].

Aim is **latched when the jab starts** (`just_started`), not at Active. Chamber
already points at the cone; look changes mid-punch do not retarget.

`ArmAim` / shoulder lift are character-space and **not** resolver-mirrored.
The target is shared for both sides (X is not flipped). Per-side tests assert
the punching hand stays on its own side (`left.x > 0`, `right.x < 0`) and
does not cross the midline.

## Timing

The authored [`Jab`](../../world/layers/mobs/characters/animations/src/animations/jab.rs)
clip is one-shot at [`JAB_DURATION_SECS`](../../world/layers/mobs/characters/motion/src/clip.rs)
(`0.42 s`). Clip envelopes are unchanged; gameplay startup ends during the
authored snap so Active sits on the incoming fist:

| Phase | Clip progress | Seconds | What |
|---|---|---|---|
| **Startup** | `0.00 .. 0.32` (`JAB_STARTUP_END`) | 0.134 s | Chamber then incoming snap |
| **Active** | `0.32 .. 0.52` (`HOLD_END`) | 0.084 s | Hit window through full-extension hold |
| **Recovery** | `0.52 .. 1.00` | 0.202 s | Return to guard |

A press during startup or active **queues one** follow-up (latest wins). A press
during recovery **cancels into** the next jab immediately. That queue/cancel
slot is the combo-graph hook.

## Gameplay / visuals

`AttackState` owns time, side, and the latched [`JabAim`](src/aim.rs). During
the active window the crate emits [`FisticuffsStrike`](src/lib.rs)
(`attacker`, `side`, `kind`, clip `progress`, quantized `target`). Damage
resolution is a TODO: there is no fist sensor, and
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

In the world / game, drop the held item, then tap LT/RT (or F/G). Look aims
the jab; RMB aims the camera without punching.

The authored clip can still be scrubbed in `characters-playground`:

```bash
cargo run -p characters-playground -- character assemble --animation jab --side left
cargo run -p characters-playground -- character playback --progress 0.47
```
