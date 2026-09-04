# Evasion intelligence

Assailant knowledge for civilians and other non-combat brains.

`EvasionIntelligenceUser` mirrors [`combat-targeting`](../combat/targeting):
source-owned membership, last-known contact snapshots, factor algebra, and a
cached ranked list. After ranking it emits an exclusive [`EvasionSignal`](src/signal.rs)
of `Idle`, `Flee`, or `Hide`. It does not write
[`MovementObjective`](../movement/lib/src/objective.rs).

Typical flow:

1. Perception calls `upsert_sighting`; this records memory and includes `SPOTTING`.
2. Roster / shot adapters call `include` or `note_stimulus` for `ENEMYSHIP` and
   `RECEIVED_FIRE`. A shot must not fabricate a successful sighting.
3. [`EvasionPlugin`](src/plugin.rs) rebalances and routes the signal by distance
   to the best actionable contact.
4. [`fleeing-intelligence`](../fleeing) or [`hiding-intelligence`](../hiding)
   consume that signal and write movement.
