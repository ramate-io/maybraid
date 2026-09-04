# Hiding intelligence

Consumes [`EvasionSignal`](../evasion/src/signal.rs) `Hide` and writes
[`MovementObjective::Reach`](../movement/lib/src/objective.rs) to a nearby
low-vantage, low-occupancy pocket. Occupancy counts live [`SpotSubject`](../spotting/lib/src/subject.rs)
bodies and [`HideClaim`](src/lib.rs) points. Concealment is a Fixed-layer
obstruction between the threat snapshot and the candidate. Search is centered
on the civilian, not on the assailant.
