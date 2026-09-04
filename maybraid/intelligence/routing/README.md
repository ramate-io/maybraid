# Routing intelligence

Long-range navigation. [`RoutingSettings::bands`](src/band.rs) are part of a
particular mover — 1000 / 500 / 100 m is only the default example.

Each band commits waypoints at its `segment` spacing. Finer bands search along
the parent corridor (with lateral slack) and score chords with:

- a long hip-height Fixed ray (buildings / walls);
- periodic downcasts (cliffs, gaps, drop above `max_fall`).

Failed fine chords feed back as extra cost on the next coarse plan. Continuity
pulls samples toward the previous polyline.

[`RoutingPlugin`](src/plugin.rs) writes [`MovementObjective::Reach`](../movement/lib/src/objective.rs)
for the current fine hop. Local walking stays on
[`movement-intelligence`](../movement/lib).
