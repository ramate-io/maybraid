# Meandering intelligence

`MeanderingIntelligenceUser` chooses a retained POI inside a nearby radius and
creates a `PoiGoal`. Selection favors relevant, salient, confident, nearby
destinations while `PoiVisitPolicy` controls novelty or an explicit cycle.

Install `PoiIntelligencePlugin` before this plugin and give users
`PoiIntelligenceUser`, `PoiKnowledge`, and `PoiVisitState` components.
`enabled` is the higher-order grant; when false, selection does not start
new goals. An NPC mixer should also remove an active `PoiGoal` when retracting.
