# Journeying intelligence

`JourneyingIntelligenceUser` probes deterministic Gimme tiles several broadphase
cells away, learns a relevant POI in an occupied tile, and creates a `PoiGoal`
for routing or movement. Confirmed-empty tiles are cached briefly.
`linger_secs` is copied onto the goal (default zero: finish on first arrival).

Sparse `GlobalPoi` scans make whole-map destinations available without requiring
every local POI to enter a global index. A cycle policy first fills its explicit
roster from distant tiles, then repeats that roster independent of later tile
probes.

If an entity has both meandering and journeying components, meandering owns POI
selection; journeying remains inactive so the two brains cannot race one goal.
