# Mob intelligence

Pack-level brain above [`npc-intelligence`](../npc). A mob host always exists.
NPC mixers exist only while a High plant is bound into a roster slot.

```text
Mob host (always)
  roster, Tether, pack affiliations, PoiInterests
  optional journey / MobTravel (moves the tether)
           ↓ bind
High plant: MobSlot + optional MobId
  Personality::install, MemberOf { mob, slot }
```

This crate does **not** pick Guard vs Occupy. Callers own kind recipes. It does
**not** implement LodScene. High fulfill later stamps the same Entity-free wish
the playground stamps today; see [ROSTER.md](ROSTER.md).

## What the host owns

- [`Mob`](src/host.rs) + [`MobId`](src/host.rs)
- [`MobRoster`](src/roster.rs): personality spec, last pose, health, live `Entity`
- [`Tether`](../tether) marker so members can leash / stalk the host
- [`MobAffiliations`](src/roster.rs) and [`PoiInterests`](../poi): copied onto members at bind
- [`MobRespawn`](src/roster.rs): delay + lives; emits [`MobMemberNeeded`](src/roster.rs)
- optional [`MobTravel`](src/travel.rs) toward a [`PoiGoal`](../poi)

Systems query the roster, not `With<Npc>` children. `npc-intelligence` stays
LodScene-ignorant.

## Bind

Plants carry [`MobSlot`](src/member.rs) and, when they are not under the host,
the host's [`MobId`](src/host.rs). Bind writes [`MemberOf`](src/member.rs) and
`roster[slot].entity`. If the mixer is missing, it calls `Personality::install`
with the host as tether subject.

The app spawns bodies. This crate never depends on character controllers.

## Playground

The [personalities playground](../personalities) is still a flat High-fulfill
stand-in. It now uses this brain (shared tether host, roster bind) instead of
stamping `MobMember { mob: host }` by hand.
