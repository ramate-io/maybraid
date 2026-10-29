# Bikanwarda Lubikan charge rules

[#1136](https://github.com/ramate-io/maybraid/issues/1136): affiliation combat weights and
Sope's Banyan forest occupancy for the equipped Bikanwarda account (Ward / Enraged fixtures).

This crate depends on [`lubikans`](../) and [`occupancy-rewards`](../../occupancy-rewards). It
does not change Self charge, healing, or the generic Self combat award in `lubikans`.

## Combat

| Victim class | Damage | Kill |
|--------------|--------|------|
| Animal (herd / pack) | −5 | −10 |
| Raider | +5 | +25 |

Classification uses [`ThreatGroupId`](../../intelligence/threat/src/id.rs) memberships aligned
with [`mob_scenes::brain`](../../world/layers/mobs/scenes/src/brain.rs). Victim class is
snapshotted on first qualifying hit so identity survives retirement before Award runs.

Generic [`LubikanCombatAward`](../src/award.rs) applies only while Self is equipped; Bikanwarda
accounts are credited here.

## Occupancy

+5 charge every five seconds in a **forest cell** that publishes
[`ConstructionTag::SOPES_BANYAN`](../../occupancy-rewards/src/tag.rs) (semantic HCSG metadata,
not rendered trees). Awards use the owner's equipped selection at tick time, like Self occupancy.

## Registration

```rust
app.add_plugins(BikanwardaLubikansPlugin::default());
```

Optional: `BikanwardaLubikansPlugin::extend_occupancy_rules(&mut RewardRuleSet)` to merge the
forest rule without replacing Self demonstration rules.
