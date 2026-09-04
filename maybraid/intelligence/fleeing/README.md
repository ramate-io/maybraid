# Fleeing intelligence

Consumes [`EvasionSignal`](../evasion/src/signal.rs) `Flee` and writes
[`MovementObjective::FleeFrom`](../movement/lib/src/objective.rs) around the
best actionable assailant snapshot. Does not rank threats or search cover.
