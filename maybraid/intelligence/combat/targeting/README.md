# Combat targeting

Shared, weapon-neutral combat contact memory and target ranking.

`CombatTargeting` deliberately separates two stores:

- `memory` contains recently visible `CombatContact` snapshots.
- `active` contains entities admitted by one or more semantic `TargetSource`
  bits. A target can remain active after its sighting expires when another
  source, such as `OBJECTIVE` or `RECEIVED_FIRE`, still admits it.

## Typical use

1. Insert `CombatTargeting` on each combatant and add
   `CombatTargetingPlugin`.
2. Perception calls `upsert_contact`; this records memory and includes
   `SPOTTING`.
3. Team, objective, and weapon adapters call `include`, `remove_source`,
   `exclude`, or `allow`.
4. Policy adapters call `set_factors` and `add_influence` for active entries.
5. Systems ordered after `CombatTargetingSystems::Rank` read `best` or
   `current`, then use `contact` when an aimable observation is required.

`exclude` masks matching source bits without deleting either the contact or
the source. `allow` reverses that mask. `TargetSource::ALL` can suppress every
membership route.

Engagement adds one continuity unit only to the temporary factors used during
ranking. It does not rewrite caller-owned `ActiveTarget::factors`.

`enabled` is the higher-order grant. When false, spotting adapters must not
admit `SPOTTING` membership and ranking clears the cached order. Entities
without a management brain stay enabled.

## Opportunity and distance

The rank system uses `Time` for memory expiry and influence decay, but does not
read the observer `Transform`. Distance is only one possible interpretation of
`opportunity`, and the shared crate does not know weapon range, cover, firing
arc, or whether vertical distance matters. Firearm or movement adapters should
derive their preferred normalized opportunity from the observer transform and
the contact/current target transform, then call `set_factors`.

`CombatContact::freshness` is a linear `0.0..=1.0` memory-window value.
`TimedInfluence::decayed_value` uses exponential half-life decay.
