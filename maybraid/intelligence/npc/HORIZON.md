# Horizon: groups, mobs, and LodScene

This crate is the NPC runtime brain. A later **mob** brain sits one level up.
LodScene should host mobs the way groves host plants, not put a mixer on every
character in the world.

## Two brains, three LOD identities

```text
Group lattice (no LodScene)
  hopscotch / softmax: world features → mob kinds
  cell ~ hundreds of metres to a kilometre
           ↓
Mob host = LodSceneHost + mob brain (always)
  roster, shared tether, affiliations, GlobalPoi
  Medium/Low: no NPC brains
  High (~200 m): personality bundles / refs
           ↓
NPC mixer + personality (only while High is shown)
```

Copy **forest / grove**, not “every NPC is a host.”

- [`ChicoForest`](../../chico/forests/CONTRIBUTING.md) is **select-only** and
  does not implement `LodScene`.
- Groves are the hosts. High grows real plants; coarser bands keep the grove
  and drop expensive leaves.
- Nested refresh is already gated on the parent showing the right level.

**Groups are forests. Mobs are groves. NPCs are High plants.**

## What stays alive when NPCs are gone

The mob brain must run off a **roster**, not live NPC queries. Occupying,
relocating a tether, and “this pack antagonizes public” cannot require twelve
`ThreatKnowledge`s in the world.

Always on the mob host:

- kind + overrides (Guard vs Roam)
- shared tether (entity or pose)
- affiliation tables
- member roster: personality spec, last pose, health, cheap summaries
- journeying / `GlobalPoi` for **moving the tether**
- respawn policy

High fulfill: for each roster slot, spawn the body and
`Personality::install(...)`. High cull: write pose/health back onto the roster
and despawn the NPC. `TetherMemory` already survives uninstall; the roster is
the same idea for members.

If the mob brain reads `Query<&Transform, With<Npc>>`, culling breaks Occupy
and Roam.

## Bands

- **High (~200 m), cull hard.** Spotting/threat interaction is ~80 m; 200 m is
  seeing the pack before it thinks. This is the only band that should run
  spotting, threat discovery, firearm, evasion, meander, idle, and the NPC mixer.
- **Medium.** Optional impostors, or empty. Do not put half-intelligence here.
- **Low / UltraLow.** Host only. The group still sees a Guard on the POI.

Do not put `LodScene` on the group. Groups pick kinds and stream mob hosts.

**Refs** are roster slots in the High chunk queue: a spec + pose, not a live
`Entity`, until spawn budget admits them.

## Feature matching (groups)

Forest layerings map biome → grove kind. Groups map world features → mob kind:

- road / corridor → Roam, Traveling monk
- prey density / wild POI → Hunt
- settlement interior → Occupy
- gate / public POI → Guard
- arena / authored pit → FFA

Personality mix is a grove recipe: Guard is mostly Brawler, Roam is Grazer +
Civilian, Hunt is Predator + Assassin.

## Mob kinds (later crate)

| Mob | Tether | Travel | Notes |
|---|---|---|---|
| Roam 1–18 | weak | journey **the tether** | members meander inside the leash |
| Hunt 1–12 | weak; members stalk | same | on Combat, inner leash; `keep_tether_in_combat` |
| Occupy 1–32 | strong, fixed | none | grazers / civilians |
| Guard 4–20 | strong, one POI | relocate on signal | antagonize public; small discovery radius |
| FFA 8–16 | strong (arena) | none | FFA affiliations; Hold until player fire |
| Traveling monk | weak | roam tether | grazer / ignore-heavy |

Translating a POI into a threat (“attack this camp”) is a **mob adapter**, not
a personality.

## What this crate must not do

Keep `npc-intelligence` ignorant of LodScene. High fulfill calls the same
`Personality` constructors the firing range uses. Mob systems query a roster,
not children.

The [personalities playground](../personalities) is a flat High-fulfill stand-in:
one host entity per proto-mob, members installed with those constructors. It is
not a LodScene and must not become the mob crate.

Failure mode: NPC brains parented to the mob host but **not** under the High
root, so they survive cull. Presence of the personality / mixer bundle **is**
“this member is in High.”
