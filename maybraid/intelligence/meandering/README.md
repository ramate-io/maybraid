# Meandering intelligence

`MeanderingIntelligenceUser` chooses a retained POI inside a nearby radius and
creates a `PoiGoal`. `linger_secs` is copied onto that goal so the mover stays
at the destination after first arrival; leaving the disk resets the clock.
Selection favors relevant, salient, confident, nearby destinations while
`PoiVisitPolicy` controls novelty or an explicit cycle. Candidates the mover
already occupies are skipped so a finished visit becomes the next other POI,
not a no-op reissue.

Install `PoiIntelligencePlugin` before this plugin and give users
`PoiIntelligenceUser`, `PoiKnowledge`, and `PoiVisitState` components.
`enabled` is the higher-order grant; when false, selection does not start
new goals. An NPC mixer should also remove an active `PoiGoal` when retracting.
