# Input

Common virtual pad for Maybraid. Bevy’s `bevy_input` stays the HID layer.
This crate maps keyboard, mouse, and gamepad onto one per-player snapshot,
then integrates a bounded cursor and a short history window. Menu controllers interpret navigation from authorized rich frames.

```text
Bevy HID  →  VirtualPad (analog + digital + keys)  →  routed menu / gameplay consumers
                 └─ PadHistory (snapshots + edges)
```

## Pad

Analog is current value. Digital is press / pressed / release (`just_pressed`,
`pressed`, `just_released`) plus optional `hold_secs`. Hold is not a third
event. Chords are queries. Text / IME stays on Bevy `KeyboardInput`.

| Field | Source |
|---|---|
| `move_stick` | Left stick, WASD / arrows (gameplay map) |
| `look_stick` | Right stick (camera-space) + mouse delta |
| `trigger_focus` / `trigger_fire` | LT / RT, digital after threshold |
| `dpad` | Hat + arrow keys |
| `buttons` | Xbox-letter face (`South` → A) |
| `keys` | Full physical `KeyCode` overlay |

Default keyboard faces (gameplay map): Space → A, Shift → L3 (sprint), Escape → B
(menu Back; character tap squat / hold prone when no menu is focused). **B** is
not a Back key — it types. Enter → Start, Tab → Select, C → left bumper
(iron ADS), E / X → X (interact). WASD walks; arrows walk and also feed the
D-Pad.

While a menu is focused or a text modal is open, capture derives a menu
[`PadInputPolicy`]: only arrows (D-Pad), Escape (Back), and Enter (Start) so
letter keys can type. Gamepad left stick still navigates menus. Console text
capture also suppresses analog / face buttons. The key overlay remains.

Gameplay systems should read sticks / buttons. `keys` is the escape hatch
(`/`, F1, chords). Hosts push or pop a capture layer; they do not write pad
lanes. The capture resolver writes [`PadInputPolicy`].

## Schedule

`VirtualPadSystems::Produce` (after Bevy `InputSystems`) then `Derive`, both
in `PreUpdate`. Consumers run in `Update`. An optional [`PadOverlay`] resource
is applied after HID producers and before `finish_produce`. Insert it only when
an injector (the agent plugin) is active; missing overlay is a no-op.

Set [`VirtualPadConfig::debug_overlay`] to dump raw `Gamepad` sticks/buttons
and the virtual pad onto the screen (menu playground enables this). Menu
stick threshold is `0.2` so a light tilt still navigates. Unmapped gilrs
axes (`Other(0)` / `Other(1)`) fill in when `LeftStick` is idle.

On macOS / iOS, chain `.with_pad_hid()` on `DefaultPlugins` so gilrs is not
started. `VirtualPadPlugin` then polls Apple’s GameController framework into
Bevy `Gamepad`. Xbox pads on macOS are claimed by that driver; IOKit (gilrs)
can connect them without delivering reports. Linux and Windows keep gilrs.

Apple GameController and Core Haptics objects are main-thread-only. The
`hid/gamecontroller.rs` backend stores them in thread-local storage and marks
its poll / rumble / init systems with Bevy `NonSendMarker` so the multithreaded
executor keeps them on the main thread.

## Rumble

Gameplay writes [`PadRumble`](src/rumble.rs); the plugin copies it onto every
live `Gamepad` as Bevy `GamepadRumbleRequest`. There is no mouse / keyboard
path. Linux and Windows play those requests through gilrs. On Apple,
`hid/gamecontroller.rs` drives `GCDevice.haptics` / Core Haptics so the same
pulse reaches a connected Xbox or DualShock.

## Source-owned runtime

`VirtualPadController` is a component owning one rich `VirtualPad`, its explicit
`PadDevices` assignment, policy, history, cursor and sequence. `VirtualPadEvent`
publishes the controller entity, sequence, update duration and an immutable rich
snapshot; receivers interpret their own menu, character or shell meaning.

The default host explicitly combines keyboard, mouse and the first gamepad into
`PrimaryVirtualPad`, preserving single-player controls. Additional gamepads get
separate controller entities and await actor assignment. Disable
`VirtualPadPlugin::auto_assign_devices` for explicit multiplayer/device setup.
A device change cancels held gameplay input until release. A missing assigned
gamepad produces release/neutral state independently of other sources.

During the consumer migration, resources mirror only `PrimaryVirtualPad` and
legacy menu interpretation remains an adapter. New consumers use source snapshots;
they must not interpret another controller's state through those resource mirrors.

Each controller also owns its `VirtualPadConfig`; `with_config` customizes an
individual controller without changing another source. Default-host reconnection
reuses vacant source entities, replaces missing physical-device references, and
requires a release before a device that reconnects while held can press again.

`pad::PadRearm` filters a rich snapshot clone after gameplay ownership is canceled,
retaining per-control release/neutral requirements while leaving the source's raw
state intact. Suppressed physical analog values remain available privately to
rearm even when text-entry policy clears the public gameplay axes.
