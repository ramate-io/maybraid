# Character controller

Maps [`VirtualPad`](../../input/README.md) onto gameplay intents. Analog is
the current stick / trigger. Digital is a press or release edge. Chords are
resolved here so consumers do not re-read the pad.

## Pipeline

Stage order is [`ControllerPipeline`](src/pipeline.rs) / [`MotorRequestStage`](src/pipeline.rs),
not plugin install order:

1. **Pad** — `VirtualPadSystems` in PreUpdate (`maybraid-input`).
2. **Bind** — `produce_character_intents` → [`CharacterIntent`].
3. **Branch** — camera look and item intents read the same bus; they do not write `MoveWish`.
4. **Motor request** — Standard `apply_move_intents`, Tanzer overwrite, world gameplay filter.
5. **Motor** — shared player-crate capsule (`PlayerSystems::Body`).

`OrdinaryLocomotionEnabled` is the pad-level gate (one local stick). Tanzer
mode still suppresses move / jump / stance at Bind so the stick can drive
recognition. Per-actor interpretation of a shared pad is a later slice.

`RightTrigger` + `X` is `PowerUseItem` and suppresses `StartInteraction`.
L3 hold is sprint (`StartSprint` every hold frame, `StopSprint` on release).
Shift maps to L3. R3 click swaps first / third person.
LT is optic `Focus`; left bumper is iron `Ads`. **B** tap is `ChangeSquat`,
**B** hold (~0.35 s) is `ChangeProne`. Menu still owns **B** as `MenuNav::Back`
when a `MenuController` has focus; character collect skips stance then.
Discover maps read stick flicks in `maybraid-skill-map`, not a bumper chord.
