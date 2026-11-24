//! Bikanwarda forest-cell occupancy rule (+5 / 5 s at Sope's Banyan).

use std::time::Duration;

use occupancy_rewards::{
	ConstructionMatch, ConstructionScope, ConstructionTag, OccupancyMatch, RewardRule,
	RewardRuleSet, RuleId, TagFilter,
};

/// Stable rule id for the Bikanwarda forest occupancy credit.
pub const BIKANWARDA_FOREST_BANYAN_RULE: RuleId = RuleId(1136);

/// +5 every five seconds in a forest cell that publishes Sope's Banyan construction metadata.
pub fn forest_banyan_rule() -> RewardRule {
	RewardRule {
		id: BIKANWARDA_FOREST_BANYAN_RULE,
		interval: Duration::from_secs(5),
		amount: 5.0,
		credited: lubikans::LubikanAccountId::SELF, // unused: adapter uses equipped selection
		matcher: OccupancyMatch {
			construction: Some(ConstructionMatch {
				tags: TagFilter::AnyOf(vec![ConstructionTag::SOPES_BANYAN]),
				scope: ConstructionScope::ForestCell,
			}),
			..OccupancyMatch::default()
		},
		overlap: occupancy_rewards::OverlapPolicy::Dedup,
		reentry: occupancy_rewards::ReentryPolicy::Reset,
	}
}

/// Merge the Bikanwarda forest rule without removing existing rules (e.g. Self demonstration).
pub fn extend_occupancy_rules(rules: &mut RewardRuleSet) {
	if rules.rule(BIKANWARDA_FOREST_BANYAN_RULE).is_some() {
		return;
	}
	rules.push(forest_banyan_rule());
}
