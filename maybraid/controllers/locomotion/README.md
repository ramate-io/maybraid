# Locomotion controller

Ordinary locomotion consumes each actor's current addressed
[`CharacterIntentFrame`](../character/src/intent.rs). Move, jump, sprint and stance
become the actor's Standard `MotorIntent` contribution. Every delivered neutral
frame clears translation; absent frames contribute nothing. Final queued writes
validate the original source/actor binding, so reassignment after Bind cannot
apply an old frame to either actor.

This crate reads no global pad, mouse buttons or routing assignment. Focus comes
from the same addressed pad interpretation as movement. Per-actor
[`ActorControlStatus`](../character/src/status.rs) preserves sprint holds during
suspension while suppressing new motion; capture/disconnect release them through
the motor lifecycle contract. `OrdinaryLocomotion(false)` suppresses the Standard
movement/jump/stance path for Tanzer mode.

This crate declares motor requests. The player crate composes contributions and
applies `MoveWish` once. Tanz and skill-map consume the same actor-owned physical
control samples. See [ARCHITECTURE.md](../ARCHITECTURE.md).
