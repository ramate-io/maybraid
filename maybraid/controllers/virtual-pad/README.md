# Virtual-pad controller

Common virtual pad for Maybraid. Bevy’s `bevy_input` stays the HID layer.
Each `VirtualPadController` entity owns its rich pad, device assignment, configuration,
policy, bounded cursor and history. The crate publishes immutable `VirtualPadEvent`
snapshots identified by that controller entity, update sequence and duration.
Menu, character and shell receivers interpret authorized frames in their own domains.

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

Addressed receivers read snapshot sticks / buttons. `keys` is the escape hatch
(`/`, F1, chords). Hosts push or pop a capture layer; they do not write pad
lanes. The capture resolver writes [`PadInputPolicy`].

## Schedule

`VirtualPadSystems::Produce` (after Bevy `InputSystems`) then `Derive`, both
in `PreUpdate`. Consumers receive addressed input in `Update`. Injectors send
`PadOverlayRequest { source, overlay: Some(pad_overlay) }` for an explicit controller
entity; `None` removes that source's injection. Requests are applied inside `Produce`
before physical sampling, so hosts write them before `VirtualPadSystems::Produce`.
Overlay storage belongs to the named controller.

Set [`VirtualPadConfig::debug_overlay`] to dump raw `Gamepad` sticks/buttons
and each enabled controller onto the screen. Unmapped gilrs axes (`Other(0)` / `Other(1)`) fill in when `LeftStick` is idle.

On macOS / iOS, chain `.with_pad_hid()` on `DefaultPlugins` so gilrs is not
started. `VirtualPadPlugin` then polls Apple’s GameController framework into
Bevy `Gamepad`. Xbox pads on macOS are claimed by that driver; IOKit (gilrs)
can connect them without delivering reports. Linux and Windows keep gilrs.

Apple GameController and Core Haptics objects are main-thread-only. The
`hid/gamecontroller.rs` backend stores them in thread-local storage and marks
its poll / rumble / init systems with Bevy `NonSendMarker` so the multithreaded
executor keeps them on the main thread.

Headless hosts and tests should insert `PadHidPolicy::External` before adding
`VirtualPadPlugin` or a gameplay plugin that installs it. This omits this crate's
hardware initialization while keeping Bevy input processing, device discovery,
and virtual-pad sampling active, so fixtures can supply `Gamepad` components,
input messages, or overlays. It does not disable separately installed hardware
plugins such as gilrs. The default `PadHidPolicy::Platform` retains the native
Apple backend and its process-main-thread requirement. Set the policy during
app construction; changing it later does not reconfigure an installed backend.

## Rumble

Gameplay writes [`PadRumble`](src/rumble.rs); the plugin copies it onto every
live `Gamepad` as Bevy `GamepadRumbleRequest`. There is no mouse / keyboard
path. Linux and Windows play those requests through gilrs. On Apple,
`hid/gamecontroller.rs` drives `GCDevice.haptics` / Core Haptics so the same
pulse reaches a connected Xbox or DualShock.

## Source-owned runtime

`VirtualPadController` owns one rich `VirtualPad`, its explicit
`PadDevices` assignment, configuration, policy, history, cursor and sequence. `VirtualPadEvent`
publishes the controller entity, sequence, update duration and an immutable rich
snapshot; receivers interpret their own menu, character or shell meaning.

The default host explicitly combines keyboard, mouse and the first gamepad into
`PrimaryVirtualPad`, preserving single-player controls. Additional gamepads get
separate controller entities and await actor assignment. Disable
`VirtualPadPlugin::auto_assign_devices` for explicit multiplayer/device setup.
A device change cancels held gameplay input until release. A missing assigned
gamepad produces release/neutral state independently of other sources.

Each controller also owns its `VirtualPadConfig`; `with_config` customizes an
individual controller without changing another source. Default-host reconnection
reuses vacant source entities, replaces missing physical-device references, and
requires a release before a device that reconnects while held can press again.

`pad::PadRearm` filters a rich snapshot clone after gameplay ownership is canceled,
retaining per-control release/neutral requirements while leaving the source's raw
state intact. Suppressed physical analog values remain available privately to
rearm even when text-entry policy clears the public gameplay axes.

The pad, policy, history, cursor and overlay values are not global resources.
`PrimaryVirtualPad` is the explicit host-selected controller identity; it does not
mirror controller state. Pure `KeyboardPad::apply`, `MousePad::apply` and
`GamepadAxes` mapping helpers share device interpretation with headless fixtures.
