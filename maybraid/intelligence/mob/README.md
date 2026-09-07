# Mob intelligence

Pack-level brain above [`npc-intelligence`](../npc). A mob host always exists.
NPC mixers exist only while a High plant is bound into a roster slot.

```text
Mob host (always)
  roster, Tether, pack affiliations, PoiInterests
  optional journey + RoutingIntelligenceUser (plans a corridor)
    MobTravel slides the tether along hops
           ↓ High stub (ChildOf, cull handle)
RosterRef { recipe, slot, offset }  — no capsule
           ↓ fulfill (unparented body)
High plant: MobSlot + MobId
  Personality::install, MemberOf { mob, slot }
```

This crate does **not** pick Guard vs Occupy. Callers own kind recipes. It does
**not** implement LodScene. High BSN carries [`RosterRef`](src/roster_ref.rs);
the app spawns the unparented body. See [ROSTER.md](ROSTER.md).

## What the host owns

- [`Mob`](src/host.rs) + [`MobId`](src/host.rs)
- [`MobRoster`](src/roster.rs): personality spec, last pose, health, live `Entity`
- [`Tether`](../tether) marker so members can leash / stalk the host
- [`MobAffiliations`](src/roster.rs) and [`PoiInterests`](../poi): copied onto members at bind
- [`MobRespawn`](src/roster.rs): delay + replacement cap; corpses remain for four seconds by default, then death emits [`MobMemberNeeded`](src/roster.rs). Replacements use a weighted nearby POI, avoid immediately repeating one POI, and fall back to a varied ring around the host. Cull only clears the live pointer.
- optional journey: [`install_mob_journeying`](src/host.rs) stamps [`RoutingIntelligenceUser`](../routing) so [`PoiGoal`](../poi) drives a coarse Fixed-layer corridor. [`MobTravel`](src/travel.rs) slides the host along those hops (including hop Y). Hosts are not [`movement_intelligence`](../movement/lib) users.
- [`MobTetherLock`](src/lock.rs): after arrival, member tethers sit on the destination entity for the goal linger, then restore to the host. Combat/Evade still own NPC movement.

Systems query the roster, not `With<Npc>` children. `npc-intelligence` stays
LodScene-ignorant.

## Bind

Live bodies carry [`MobSlot`](src/member.rs) and the host's [`MobId`](src/host.rs).
High BSN stamps a [`RosterRef`](src/roster_ref.rs) stub instead of parenting the
capsule. Fulfill spawns the body in world space; bind writes [`MemberOf`](src/member.rs)
and `roster[slot].entity`. If the mixer is missing, it calls `Personality::install`
with the host as tether subject.

The app spawns bodies. Journeying hosts plan corridors against Fixed colliders;
this crate still does not spawn character controllers on the tether.

## Playground

The [personalities playground](../personalities) is a flat High-fulfill stand-in
for individual mixers. The [mob-brain playground](../mob-brain) journeys hosts
and **locks** member tethers onto the destination (waypoint or prey host) after
arrival.
