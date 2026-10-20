# World controller

Owns [`WorldGameplayEnabled`](src/enabled.rs): the live-play flag that
gates character intents, combat HUD, skill-map sync, inventory cycling,
and Lubikan activation.

`WorldGameplayEnabled::accepts_character_input` is live play with no text
console and no world map sheet. `WorldGameplayEnabled::combat_hud_visible`
is the same without the console (vitals stay up while typing).

The game shell's `SessionActivity::from_shell` is the single derivation for
gameplay, physics, streaming, scenery, and menu-shell. Those are independent
bools — not one game-mode enum. [`ActivityPolicySystems`](src/lib.rs) is the
Update owner after Start toggles pause; `apply_shell_look` applies the same
table on flow enter. Pause is not a play-session teardown.
