# POI intelligence

Shared point-of-interest discovery and retained knowledge.

- `Poi` carries a stable `PoiId`, semantic `PoiKind`, arrival radius, and salience.
- `LocalPoi` enters a Gimme-backed bounded index; `GlobalPoi` enters a sparse whole-map set.
- `PoiIntelligenceUser` scans both sets according to its interests and learning policy.
- `PoiKnowledge` unions source ownership, retains old findings, and accepts directed
  `PoiObservation` messages from any other discovery system.
- `PoiGoal` hands a selected destination to routing when available, otherwise directly
  to movement. `PoiGoalState` preserves generation-tagged completion state and
  `PoiGoalCompleted` notifies one-shot consumers on arrival.

`PoiVisitPolicy::Cycle` deliberately uses a roster and cursor. Once its roster reaches
the requested size, higher-order users repeat that exact sequence instead of trying to
approximate a loop with novelty and retention weights. `Weighted` still prefers unvisited
and off-cooldown POIs (new discoveries win immediately). Cooldown is a ranking bias, not
a freeze: if every known candidate is cooling, selection keeps the stalest one so a small
local cluster keeps circulating while the next scan can load more.
