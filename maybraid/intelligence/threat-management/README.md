# Threat management intelligence

Exclusive Ignore | Evade | Combat grant between retained
[`ThreatKnowledge`](../threat) and the combat / evasion actuators.

This crate does not classify who is a threat. Classification stays in
[`threat-intelligence`](../threat). Management decides whether to **act** on
the current set.

- Combat writes known threat entities into [`CombatTargeting`](../combat/targeting)
  as `ENEMYSHIP` and inserts [`CombatSelected`](src/tactic.rs).
- Evade does the same for [`EvasionIntelligenceUser`](../evasion) assailants
  and [`EvadeSelected`](src/tactic.rs).
- Ignore retracts both grants. Spotting, POI, and meander may still run.

`FirearmEngagement` stays orthogonal: Combat + Hold still aims; Combat +
WeaponsFree may shoot.

Scoring uses two shared axes and signed coefficients:

```text
score(mode) = by_health * remaining_hp + by_distance * proximity
```

`proximity` is `1 / (1 + nearest_known_xz / horizon)` from knowledge, or `0`
when the set is empty. Empty knowledge always forces Ignore, even if committed.

Commitment `(new, old)` is the required ratio to leave the current tactic
while threats remain. `(1.0, 0.0)` never leaves Combat or Evade. Leaving
Ignore does not use that gate, so an FFA profile can still enter Combat.

Unavailable actuators are skipped: no `CombatTargeting` means Combat is not a
candidate; no `EvasionIntelligenceUser` means Evade is not.
