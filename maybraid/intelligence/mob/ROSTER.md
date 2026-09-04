# Roster bind (later LodScene)

High scene data must stay **Entity-free**. Membership is still a roster slot plus
a live pointer; the pointer is a **patch** after spawn, not a field on
`SemanticLodScene`.

## Why not bake `Entity` into LodScene

`scene_with_level` builds Bevy `Scene` / BSN from `LodRef` and a band. Those
recipes cannot name a host `Entity` without a context argument such as
`scene_with_level(parent_chain, level)`. That would make every grove plant
depend on ECS identity. Vegetation and buildings do not do this.

Drain already knows the host (`LodChunkFulfillment::host`) and parents each
primitive under the High root. That is a **spawn-time fact in the world**, not
something the scene trait should take as input.

Do not:

- add `parent_chain: Vec<Entity>` to `SemanticLodScene`
- use `ChildOf` as gameplay membership (LodScene already owns that tree)
- bake a fixed parent depth
- rebind on every lod refresh
- send a `WantsParentRelationship` message as the membership graph

`ChildOf` remains scaffolding: host → level-roots bag → `LodLevelRoot(High)` →
plants. HORIZON already forbids `Query<&Transform, With<Npc>>` off that tree;
cull would break Occupy.

## Wish in, live `Entity` out

This is the SceneRef shape: a path is not an entity yet; fulfill writes the
handle cache. A High plant's wish is a **slot**, optionally a stable [`MobId`](src/host.rs).
The roster's `entity: Option<Entity>` is the resolved cache.

```text
LodScene High BSN (pure)
  body + MobSlot(n) [+ MobId]
  no host Entity

drain_chunk_lod_fulfill
  spawn_scene → ChildOf(High root)
  host Entity is known here, but stays out of the recipe

MobSystems::Bind  (Added / missing MemberOf)
  resolve host
  MemberOf { mob, slot }
  roster[slot].entity = Some(plant)
  Personality::install(..., tether: host) if mixer missing
  copy MobAffiliations + PoiInterests

High cull
  write pose / health onto the roster
  despawn the plant
  roster[slot].entity = None
```

Fulfill and cull are the only times the live link changes. Refresh must not
rebind. Trickle spawn is the chunk budget; `LodLazyPending` is not required for
the link.

## Two resolve paths

1. **Id bind (always works).** The plant carries `MobSlot` + `MobId`. Bind looks
   up the host with that id. Use this when the body is **not** a child of the
   host (playground today; characters that must sit under a High root for cull
   but are not gameplay children).
2. **Ancestor bind (LodScene drain).** The plant carries `MobSlot` only. Bind
   walks `ChildOf` to the nearest ancestor [`Mob`](src/host.rs). Walk to `Mob`,
   **not** `LodSceneHost` — a character host on the NPC would win first.

Prefer an explicit `MobId` when both are present.

Playground `Commands` that already hold `host: Entity` may stamp `MemberOf` in
the same batch. That path never needed LodScene context. The bind system exists
so serialized High content can stay as dumb as every other grove plant.

## What stays on the host while plants are gone

The mob brain runs off the roster: spec, last pose, health, cheap summaries.
Occupying, relocating the tether, and pack antagonism cannot require live NPC
queries. Respawn is a [`MobMemberNeeded`](src/roster.rs) message; the app spawns
the body and stamps the wish again.

Journeying / `MobTravel` move the **host** (the tether). Members follow through
tether intelligence, not by parenting.

## Character trickle

NPCs can appear one-per-frame under the fulfill budget. Bind is incremental:
each new `MobSlot` without `MemberOf` is one plant. No extra relationship sync
and no parent-wish dirty flag.
