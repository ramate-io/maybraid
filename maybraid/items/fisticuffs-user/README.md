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
must not throw a jab. Non-mouse trigger edges remain independent while a mouse
button is held, including after a capture ends.

Jabs only start while **Fisticuffs** is the equipped primary tool. The world
equipment adapter inserts [`FisticuffsUser`](src/lib.rs) for that selection and
removes it (canceling a pending strike) when another item is queued. An empty
weapon queue does not enable punches. Held guns and grenades keep LT/RT as
focus/fire.

## Aim

[`PlayerLook`](../../player/lib/src/identity.rs) is camera **−Z** forward. The posed
humanoid is character space (**+Z** fight-forward, **+X = character left**).
**Look left** is camera **+yaw** (player view). Camera-to-character is a
**180° yaw** (negate X and Z). [`JabAim`](src/aim.rs) converts look into that
frame, clamps to a punching cone, and quantizes:

| Axis | Clamp | Step | Buckets |
|---|---|---|---|
| Pitch | −24° (belly) … +36° (above head) | 12° | −24, −12, 0, +12, +24, +36 |
| Yaw | ±24° off body facing | 12° | −24, −12, 0, +12, +24 |

Level look maps to `DEFAULT_JAB_TARGET`. The same quantized character-space
point is written to `JabParams.target` and [`FisticuffsStrike::target`].

Aim is **latched when the jab starts** (`just_started`), not at Active. Chamber
already points at the cone; look changes mid-punch do not retarget.

`ArmAim` is character-space and **not** resolver-mirrored. The punching arm
lerps onto the look direction as it extends, so shoulder→tip tracks the
quantized angle at Active. Level jabs sit outboard of the midline (`left.x > 0`,
`right.x < 0`). A full opposite-side yaw is a cross-body shot.

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
slot is the combo-graph hook. A tick that would skip the entire active window
stops at its entry for one observable frame, allowing one strike and hit sensor
before recovery continues on the next tick.

## Gameplay / visuals

`AttackState` owns time, side, and the latched [`JabAim`](src/aim.rs). During
the active window the crate emits [`FisticuffsStrike`](src/lib.rs)
(`attacker`, `side`, `kind`, `combo_step`, clip `progress`, quantized `target`)
and spawns a short-lived fist / forearm [pose marker](src/sensor.rs) from the
character and aim. The marker carries no Avian collider. A swept capsule runs
from the chamber area to the intended fist endpoint, querying Fixed and
Animated solids. Its first contact emits [`FisticuffsImpact`](src/lib.rs) with
the struck collider and world surface point/normal. A jab can land once; a
solid without Health still stops damage reaching anything behind it. A Health
owner receives
`damage::Hit { source: Some(attacker), allow_self: false, amount: PLACEHOLDER_JAB_DAMAGE }`.
[`HitCapsule`](../../player/lib/src/hit.rs) stays receive-only.
[`maybraid-audio`](../../audio/) plays the same authored damage flinch grunt
the hurt character already uses (`FlinchSounds::play`, `Voices` bus) and plays
a spatial landing thump from the impact event on the `Impacts` bus.

The sensor exists **only** while [`AttackPhase::Active`](src/attack.rs). Cancel,
weapon swap (`FisticuffsUser` removed), death / `Downed`, and routing capture,
disconnect, or reassignment tear it down. Source suspension freezes the assigned
actor’s clock and hit resolution; other actors continue. Resume continues the
same attack, retaining its consumed-contact latch. Impacts and damage are once
per strike. There is
no combo-step damage scale — `combo_step` is reserved for a later combo graph.

`PLACEHOLDER_JAB_DAMAGE` (12) and the capsule radius / length / reach are
**not final balance**. `FisticuffsSensorDebug(true)` draws the live volume
(world playground chrome turns it on). Player-sourced `DamageApplied` already
raises combat-hud `+1` / `+2` / `+5` hit markers.

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

In the world / game, equip **Fisticuffs** from Inventory → Tools (it is an
intrinsic, non-droppable bag item), then tap LT/RT (or F/G). Default starter
loadouts queue firearms and a grenade first; Fisticuffs stays in the bag until
selected. Look aims the jab; RMB aims the camera without punching. Switching
to a gun or grenade cancels a chambered punch before it can fire.

The authored clip can still be scrubbed in `characters-playground`:

```bash
cargo run -p characters-playground -- character assemble --animation jab --side left
cargo run -p characters-playground -- character playback --progress 0.47
```

Headless jab-hit stills (sensor volume + combat-hud marker on a dummy):

```bash
FISTICUFFS_JAB_CAPTURE_DIR=/tmp/fisticuffs-jab \
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  WGPU_BACKEND=vk \
  xvfb-run -a cargo run -p menu-playground --example fisticuffs_jab_hit_capture
```

## Runtime and presentation

`register_fisticuffs_user_runtime` installs the attack machine, equipment
hold/teardown, strike and impact messages, and hit resolution once. Headless gameplay uses
this same runtime without audio or mesh assets. Hit resolution runs when the host
has physics; attack timing and sensor lifetime still run without it.

`FisticuffsUserPlugin` adds clip presentation, debug drawing, and strike vocals
on top of that runtime. Installing it after gameplay does not register a second
attack machine.

Jab input consumes addressed `ActorIntentEvent` values. Pending requests retain
`Routed` provenance and are validated immediately before starting after the
attack tick, so retired input cannot start a punch and chamber-time aim remains
observable. `ActorRoutingEvent` cancellation applies only to the actor that lost
delivery. Per-actor `ActorControlStatus` freezes attack time and damage during
suspension; autonomous actors continue independently.
