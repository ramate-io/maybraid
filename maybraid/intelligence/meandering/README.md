# Meandering intelligence

`MeanderingIntelligenceUser` chooses a retained POI inside a nearby radius and
creates a `PoiGoal`. Selection favors relevant, salient, confident, nearby
destinations while `PoiVisitPolicy` controls novelty or an explicit cycle.

Install `PoiIntelligencePlugin` before this plugin and give users
`PoiIntelligenceUser`, `PoiKnowledge`, and `PoiVisitState` components.
