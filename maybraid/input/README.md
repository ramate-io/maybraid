# Input

Common virtual pad for Maybraid. Bevy’s `bevy_input` stays the HID layer.
This crate maps keyboard, mouse, and gamepad onto one per-player snapshot,
then derives menu navigation, a bounded cursor, and a short history window.

```text
Bevy HID  →  VirtualPad (analog + digital + keys)  →  MenuNav / Cursor / gameplay
                 └─ PadHistory (snapshots + edges)
```

## Pad

Analog is current value. Digital is press / pressed / release (`just_pressed`,
`pressed`, `just_released`) plus optional `hold_secs`. Hold is not a third
event. Chords are queries. Text / IME stays on Bevy `KeyboardInput`.

| Field | Source |
|---|---|
| `move_stick` | Left stick, WASD / arrows |
| `look_stick` | Right stick (camera-space) + mouse delta |
| `trigger_focus` / `trigger_fire` | LT / RT, digital after threshold |
| `dpad` | Hat + arrow keys |
| `buttons` | Xbox-letter face (`South` → A) |
| `keys` | Full physical `KeyCode` overlay |

Default keyboard faces: Space → A, Escape → B, Enter → Start, Tab → Select.

Gameplay systems should read sticks / buttons. `keys` is the escape hatch
(`/`, F1, chords). Set [`PadGameplayEnabled`] to `false` while text is focused
so analog and pad buttons clear; the key overlay remains.

## Schedule

`VirtualPadSystems::Produce` (after Bevy `InputSystems`) then `Derive`, both
in `PreUpdate`. Consumers run in `Update`.

Set [`VirtualPadConfig::debug_overlay`] to dump raw `Gamepad` sticks/buttons
and the virtual pad onto the screen (menu playground enables this). Menu
stick threshold is `0.2` so a light tilt still navigates. Unmapped gilrs
axes (`Other(0)` / `Other(1)`) fill in when `LeftStick` is idle.

On macOS / iOS, chain `.with_pad_hid()` on `DefaultPlugins` so gilrs is not
started. `VirtualPadPlugin` then polls Apple’s GameController framework into
Bevy `Gamepad`. Xbox pads on macOS are claimed by that driver; IOKit (gilrs)
can connect them without delivering reports. Linux and Windows keep gilrs.
