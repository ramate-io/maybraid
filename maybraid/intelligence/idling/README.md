# Idling intelligence

Cheap local puttering when Ignore has nothing better to do. Writes
[`MovementObjective::Reach`](../movement/lib/src/objective.rs) to a noisy point
inside a disk around a home origin. Does not discover POIs or classify threats.

Meandering should win whenever it can start a `PoiGoal`. This brain queries
`Without<PoiGoal>` and sits below meander in the NPC mixer: Combat / Evade
retract `enabled`, and tether still overwrites if the leash is unsatisfied.
Use it when nearby POIs are sparse, on visit cooldown, or not yet learned.
