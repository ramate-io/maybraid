# Scripted boss HCSG and runtime chain ownership

Proposal-only note for human architectural review. It names **existing**
repository APIs and a first slice; it does not implement them.

**Grounded on** [`origin/main` @ `7235124de`](https://github.com/ramate-io/maybraid/commit/7235124de)
(merged into this branch) plus the open material contract in
[PR #1145](https://github.com/ramate-io/maybraid/pull/1145)
(`cursor/mythical-chain-material-f9d3`). Names below are illustrative; Liam
owns implementation of [#1116](https://github.com/ramate-io/maybraid/issues/1116),
[#1125](https://github.com/ramate-io/maybraid/issues/1125),
[#1126](https://github.com/ramate-io/maybraid/issues/1126), and the final
[#1133](https://github.com/ramate-io/maybraid/issues/1133) GLBs. Do not take
those issues over from this document.

## These lanes are not Self prerequisites

[#1116](https://github.com/ramate-io/maybraid/issues/1116) and
[#1125](https://github.com/ramate-io/maybraid/issues/1125)–[#1127](https://github.com/ramate-io/maybraid/issues/1127)
must **not** become prerequisites for generic Self gameplay.

Self already has its own identity, ledger, charge, activation, and heal loop
([`maybraid/lubikans`](../../../../lubikans/README.md),
[#1115](https://github.com/ramate-io/maybraid/issues/1115)). Charge awards
read generic [`DamageApplied`](../../../../damage/src/lib.rs). None of that
needs a boss scheme, a chain mesh, or Bikanwarda unlocks.

Do not gate `LubikansPlugin`, HUD, or Self activation on encounter generation
or chain runtime. Summons and Enraged/Ward unlocks stay downstream of
[#1121](https://github.com/ramate-io/maybraid/issues/1121), not of Self.

---

## 1. Scripted boss / HCSG generation ([#1116](https://github.com/ramate-io/maybraid/issues/1116))

Represent authored boss encounters through the shared HCSG runtime. Leave a
typed seam for a later generated-boss scheme. **Do not** add a parallel boss
registry, spatial index, or world store.

### Existing APIs to reuse (do not fork)

| Job | Existing API | File |
|---|---|---|
| Scheme + worker generate | [`GenerationScheme`](../../../../lod/lib/src/hcsg/context.rs) / [`GenerationContext`](../../../../lod/lib/src/hcsg/context.rs) | [`lod/lib/src/hcsg/context.rs`](../../../../lod/lib/src/hcsg/context.rs) |
| Immutable store | [`HcsgStorage::seed`](../../../../lod/lib/src/hcsg/storage/writes.rs) / `publish` / `get` | [`lod/lib/src/hcsg/storage/writes.rs`](../../../../lod/lib/src/hcsg/storage/writes.rs) |
| Authored session root | `lod::seeded_root!` + [`register_session_seed`](../../../../lod/lib/src/hcsg/session.rs) | [`lod/lib/src/hcsg.rs`](../../../../lod/lib/src/hcsg.rs), [`session.rs`](../../../../lod/lib/src/hcsg/session.rs) |
| Authored catalog pattern | [`AuthoredDevelopments`](../../../urbanization/richmond/src/developments/site.rs) / [`PoiCatalog`](../../../../intelligence/poi/src/scheme.rs) | Richmond site + POI catalog |
| Discovery in a region | `original_ids_for` filters the seeded catalog; `build_with_id` is a pure lookup + ground sample | Same two files |
| Ground dependency | [`GroundSurface<G>::generate`](../../../vegetation/chico/src/hcsg.rs) / `.height` / `.footprint` | [`chico/src/hcsg.rs`](../../../vegetation/chico/src/hcsg.rs) (Barking already does this) |
| Unique hosts + stream-out | [`PresentationPlugin<C, T>`](../../../../lod/lib/src/hcsg/presentation.rs) / [`HcsgNode<T>`](../../../../lod/lib/src/hcsg/node.rs) | One host per `Id`; retire by stored bounds |
| World channel | [`InDiscovery<MobNeighborhood>`](../../../lib/src/world_layers.rs) or a sibling `EncounterNeighborhood` on the same `HcsgBoundsPlugin` pattern | [`world_layers.rs`](../../../lib/src/world_layers.rs) |
| Session restart | [`HcsgRestartRequest`](../../../../lod/lib/src/hcsg/session.rs) on Discovery enter (already wired) | [`world_layers.rs`](../../../lib/src/world_layers.rs) |
| Actor recipe | [`CharacterRecipe`](../characters/lib/src/components.rs) / [`Epiphant`](../characters/lib/src/species/epiphant.rs) / [`spawn_fixed_character_assembly`](../characters/lib/src/fixed.rs) | Character crate; placeholder species until Bikanwarda art |
| Column / anchors | [`spawn_doric_column_encounter`](../architectural/doric-column/src/spawn.rs), [`DoricColumnChainAnchor`](../architectural/doric-column/src/anchors.rs) | [#1133](https://github.com/ramate-io/maybraid/issues/1133) kit; placeholders until Liam’s GLBs |
| World spawn policy (for #1117, not this slice) | [`ModePlayerPolicy::home`](../../../player/src/policy.rs) is `Vec2::ZERO` in Discovery; [`PlayerSpawnXz`](../../../lib/src/start.rs) is the player, not the encounter | [`discover/src/lib.rs`](../../../modes/discover/src/lib.rs) |

Design contract already written in
[`lod/lib/src/hcsg/README.md`](../../../../lod/lib/src/hcsg/README.md): a stored
value is a **pure function of `(TypeId, Id)` and session roots**; mutable
gameplay never lives in the `Arc`.

### Proposed types (against those APIs)

Keep one catalog root and one presented scheme. Mirror
[`AuthoredDevelopments`](../../../urbanization/richmond/src/developments/site.rs)
+ [`PoiCatalog`](../../../../intelligence/poi/src/scheme.rs), not Barking’s
lattice cells.

```text
ScriptedEncounters          // session root, seeded at Id::Universal
        │  original_ids_for: authored rows whose bounds overlap the region
        ▼
Scripted<BossEncounter>     // GenerationScheme; Id is the encounter identity
        │  build_with_id: catalog lookup + GroundSurface + immutable recipe
        ▼
HcsgNode<Scripted<BossEncounter>>
        │  LodScene: column + actor recipe + chain sockets (placeholders OK)
        ▼
ECS runtime (not in the Arc): Health, chain durability, phase, unlocks
```

- **`ScriptedEncounters`**: `Resource` + `lod::seeded_root!`. Vec of authored
  rows. Seeded in `HcsgSessionSeed` next to Richmond / Chico / Geneva.
  `HcsgStorage::seed(catalog, universal_bounds())`. Never a second store.
- **`Scripted<T>`**: newtype wrapper that marks “this `T` came from the
  authored catalog.” A later `Generated<T>` (or a second scheme) can build the
  same `T` from a procedural origin. Shared runtime (attacks, presentation
  scene, chain sockets) binds on `T`, not on the wrapper.
- **`BossEncounter`**: immutable generated value. Identity, posed bounds,
  ground height used, actor recipe handle, attack config, column spec, chain
  socket names. **No** health, durability, phase, or unlock flags.
- **Identity**: `lod::gen::Id::Bytes` derived from a stable authored slug
  (e.g. `bikanwarda.chained.v0`). Not `Id::from_cell(bounds)` — bounds edits
  must not change the key [#1121](https://github.com/ramate-io/maybraid/issues/1121)
  will persist. `Id::Bytes` has no `origin_cell_bounds()`; discovery **must**
  filter the catalog by stored AABB the way [`PoiCatalog`](../../../../intelligence/poi/src/scheme.rs)
  filters by pin position.

`Scripted<T>` is a typed generation scheme, not a registry.

### Data flow: placing one scripted boss

```text
Session start / Discovery enter
  WorldLayersPlugin → request_hcsg_session_restart
  HcsgSessionSeed → seed ScriptedEncounters (and existing Durham/Richmond/…)

Viewer regions (InDiscovery<MobNeighborhood> or EncounterNeighborhood)
  PresentationPlugin<C, Scripted<BossEncounter>>
    subscribe(C.regions) → worker
      original_ids_for: catalog rows overlapping region
      build_with_id(id):
        row = ScriptedEncounters.get(id)?          // missing catalog → None
        surface = GroundSurface<G>::generate(cx, column_bounds(row.footprint))
        height = surface.height(row.xz)?           // missing ground → None
        value = BossEncounter { id, posed_at, recipe, column, sockets, attacks }
        publish(id, Arc<value>, encounter_aabb)

  presentation: one HcsgNode per Id; skip already-hosted same version
  LodScene::scene_with_level:
    spawn_doric_column_encounter (placeholder or GLB)
    spawn_fixed_character_assembly from the authored recipe (Epiphant stand-in OK)
    stamp chain sockets from DoricColumnChainAnchor names
    do NOT insert Health / durability / unlocks into the generated value

Stream-out
  host bounds miss C.regions → RetiredHost → budgeted Last despawn
  stored Arc may evict by reach; it regenerates identically
  presentation teardown is NOT death and NOT an unlock (#1121)

Session restart
  epoch + clear_derived + reseed catalog
  hosts replace by version; #1121 progress (outside HCSG) is untouched
```

**Spawn placement.** Authored row carries world XZ (and yaw). Generation
samples [`GroundSurface`](../../../vegetation/chico/src/hcsg.rs) once and
bakes the posed Y into the published value. No post-spawn corrective scan
([#1117](https://github.com/ramate-io/maybraid/issues/1117) will choose the
XZ relative to Discovery `home = (0,0)`; this issue only defines the seam).

**Persistence.** HCSG persists *geometry of the authored placement* for the
session, not encounter outcomes. Outcomes live in an ECS / save key keyed by
the same `Id::Bytes` ([#1121](https://github.com/ramate-io/maybraid/issues/1121)).
There is no disk format in this slice; the repository’s character save path
does not yet cover world encounter progress.

**Streaming.** Unique by `Id`. Re-entry respawns the host from the same
published value. Duplicate prevention is `PresentationPlugin`’s `hosts:
HashMap<Id, …>`, the same mechanism Barking uses for `PlacedMobCell`.

### Generated-boss seam (show, do not build)

```text
impl GenerationScheme for Scripted<BossEncounter> { /* catalog ids */ }

// Later, not this issue:
impl GenerationScheme for Generated<BossEncounter> { /* lattice / noise ids */ }
// Both publish BossEncounter (or a shared presented wrapper).
```

Intent stays in **types**, as the HCSG README requires. No mode flag on a
shared store.

### Open policy questions ([#1116](https://github.com/ramate-io/maybraid/issues/1116))

| # | Question | Recommendation |
|---|---|---|
| B1 | Encounter `Id`: `Id::Bytes(slug)` vs `Id::from_cell(bounds)` like Richmond authored sites? | **`Id::Bytes` from a stable slug.** Bounds will move during [#1117](https://github.com/ramate-io/maybraid/issues/1117); [#1121](https://github.com/ramate-io/maybraid/issues/1121) must not lose the key. Discovery filters the catalog (POI pattern), not `origin_cell_bounds()`. |
| B2 | Which presentation channel? Reuse `InDiscovery<MobNeighborhood>` (1 km) or add `EncounterNeighborhood`? | **Add `EncounterNeighborhood`** (Near, ~1 km, same `ViewerHcsgBounds` style as [`MobNeighborhood`](../barking/src/hcsg.rs)). Bosses are not mob cells; sharing Barking’s scheme type would force them into `PlacedMobCell`. Sharing only the *bounds producer* is fine; a dedicated channel keeps retirement independent. |
| B3 | Where does the catalog crate live? | **`maybraid/world/layers/mobs/encounters`** (new crate, sibling of `barking`). Depends on `lod`, `chico`/`durham` ground, `doric-column`, character recipes. No `lubikans` dependency. |
| B4 | Actor for the first slice? | **Authored `CharacterRecipe` stand-in** (Epiphant / quadruped is fine). Do not block on Bikanwarda GLBs. Runtime attacks can be empty config. |
| B5 | Does generation flatten terrain? | **No in #1116.** Bake posed height onto existing ground. Flattening, if wanted, is an authored Richmond pad or a later Durham dependency owned by [#1117](https://github.com/ramate-io/maybraid/issues/1117). |

### First slice ([#1116](https://github.com/ramate-io/maybraid/issues/1116) only)

1. `ScriptedEncounters` root + `seeded_root!` + `register_session_seed`.
2. `Scripted<BossEncounter>` scheme: one hardcoded Bikanwarda row (identity,
   XZ, footprint, recipe, column spec, socket names). `None` if ground missing.
3. `PresentationPlugin<InDiscovery<EncounterNeighborhood>, Scripted<BossEncounter>>`
   registered from `WorldLayersPlugin` the same way Barking is.
4. Placeholder `LodScene`: Doric encounter spawn + fixed character assembly.
   No chain mesh, no `Health` in the `Arc`, no unlocks.
5. Tests (worker + `settle_presentation`, copy Barking’s
   [`hcsg.rs` tests](../barking/src/hcsg.rs)): identity/bounds, missing ground
   → no host, leave-neighborhood retires the host, session restart replaces
   by version and does not emit completion.

Out of slice: spawn-anchor policy ([#1117](https://github.com/ramate-io/maybraid/issues/1117)),
outcomes ([#1121](https://github.com/ramate-io/maybraid/issues/1121)), chain
([#1125](https://github.com/ramate-io/maybraid/issues/1125)–[#1127](https://github.com/ramate-io/maybraid/issues/1127)),
final GLBs ([#1133](https://github.com/ramate-io/maybraid/issues/1133)).

---

## 2. Runtime chain mesh, motion, and breakage ([#1125](https://github.com/ramate-io/maybraid/issues/1125)–[#1127](https://github.com/ramate-io/maybraid/issues/1127))

One deformable chain: generated mesh + runtime armature + capsules +
durability. Separate from Bikanwarda’s character rig
([`skin.rs`](../characters/lib/src/skin.rs) remaps *imported* clothing onto
an existing `BoneMap`; a chain does not use that path).

[PR #1145](https://github.com/ramate-io/maybraid/pull/1145) already owns
`ChainMaterial`, `OwnedChainMaterial` / `OwnedChainHandle`,
`link_damage[32]` indexed by link id, `apply_chain_hit`, and a `ChainHalo`
child. This proposal gives that material a **stable skinning contract** and
an ownership map so A04–A06 do not fight the shader.

### Ownership

One **chain root** entity owns every generated and runtime object. Nothing
below is parented to the boss character or the Doric assembly except by
**socket attachment** (follow a `DoricColumnChainAnchor` / character socket
transform). Despawn or `RetiredHost` on the chain root retires the lot.

| Object | Owner | Created by | Retired with |
|---|---|---|---|
| Joint entities (`Name` `chain_joint_{i}`, `Transform`) | Chain root children | [#1125](https://github.com/ramate-io/maybraid/issues/1125) mesh bind | Chain root |
| `SkinnedMesh` + `Handle<SkinnedMeshInverseBindposes>` | Mesh entity (child of root) | #1125; cache the mesh/bindpose assets by `(length, segments, radius)` | Chain root; drop unused handles |
| Skin bindings (`ATTRIBUTE_JOINT_INDEX` / `WEIGHT`) | The generated `Mesh` asset | #1125; never rewritten per frame | Asset cache |
| Capsule colliders (one per link, `ChildOf` that link’s joint or the root) | Chain root | [#1126](https://github.com/ramate-io/maybraid/issues/1126) | Chain root; also on break |
| Chain `Health` (durability) | Chain root | [#1127](https://github.com/ramate-io/maybraid/issues/1127) | Chain root. **Not** the boss `Health` |
| `ChainLinkId(u32)` on each capsule | That capsule | #1126 | Capsule |
| `OwnedChainMaterial` + `OwnedChainHandle` | Mesh entity | #1145 runtime | Chain root |
| `ChainHalo` child (`Mesh3d` **+ `SkinnedMesh` clone**) | Mesh entity | #1145 halo sync, extended by this contract | Mesh entity |
| Pose / tension state | Chain root component | #1126; prescribed first | Chain root |
| Break / release flag | Chain root / #1121 key | #1127 writes once; #1121 reads | Persisted outside HCSG |

**Damage identity.** Capsule hits remount through existing
[`Health::entity_or_parent`](../../../../damage/src/apply.rs) onto the **chain
root** `Health`. Boss `Health` is a sibling, not a parent. Chain death frees
Bikanwarda; it does not `Died` her.

**Retirement.** Presentation stream-out despawns the encounter host, which
despawns the chain root, which despawns joints / capsules / halo. That is
not a break and not a kill. #1121 must restore “already broken / already
killed” when the host returns.

### Stable skinning contract (for [PR #1145](https://github.com/ramate-io/maybraid/pull/1145))

#1145’s current placeholder mesh uses **UV0.x around**, **UV0.y along the
chain in `[0,1]`**, and **vertex-color R = link id**. Glow, fade, and
`impact.x` are all normalized along UV0.y. Crack sampling is link-local
(`vec2(u, v * N - link_id)`).

A04’s generated mesh should publish a contract that does **not** overload
UV0.y as both “link-local crack V” and “world-length flow.” Recommended
attributes:

| Attribute | Meaning | Encoding | Who reads it |
|---|---|---|---|
| **UV0.xy** | Around-link mapping | `u` = around the tube `[0,1]`, seam **duplicated** (`U=0` and `U=1` on coincident verts). `v` = **link-local** along the ring `[0,1]`, ring seam duplicated. | Crack atlas, twist, metal detail. Independent of chain length. |
| **UV1.x** | Rest-space longitudinal flow | Arc length in **metres** from the start anchor along the bind-pose centreline. Monotonic, not normalized. `UV1.y` reserved (`0`). | Pulse / energy flow, fade, `impact` hotspot. Does not stretch when A04 changes length or segment count. |
| **Vertex color R** | Link id | `link_index as f32` (0, 1, …), **flat** across the link so interpolation stays integer. Shader: `floor(color.r + 0.5)` indexes `link_damage[]`. | #1145 uniforms, #1127 hit routing |
| **Vertex color B** | Twist phase | `0` even / `1` odd (keep #1145). | Link orientation |
| **Vertex color G/A** | Legacy / reserved | G ignored while `localized_blend == 0`. | — |
| **JOINTS / WEIGHTS** | Skinning | Rigid per link (weight `1` on joint `i`, `0` elsewhere) **or** 2-joint blend at the ring seam. Same index space as the joint entities. | Bevy `#ifdef SKINNED` (already in #1145) |
| **Inverse bindposes** | Rest joint → mesh | One pose per joint; generated once. | `SkinnedMesh` |

**Do not** encode link id only in UV0.y. #1145 already falls back to
`floor(uv.y * link_count)` when color is missing; A04 must write color R so
that fallback is unused.

**#1145 integration change (material owner, not A04):**

1. Sample flow / pulse / fade from **UV1.x (metres)** instead of UV0.y.
2. Keep crack UVs on UV0 (around × link-local).
3. Store `impact.x` in the **same space as UV1.x** (metres), or convert
   `link_id` → rest-arc midpoint in metres inside `apply_chain_hit`.
   Today `ChainRuntimeDamage::hit` writes `impact_pos = (link_id + 0.5) / N`
   in `[0,1]` — that must change when UV1.x is metres, or the flash will
   miss on any length ≠ the placeholder.
4. Keep `link_damage[link_id]` indexed by color R. Cap remains
   `MAX_CHAIN_LINKS` (32) until someone raises it; A04 must not emit more
   without a material bump.

### Halo shell follows the skinned mesh

[#1145 `halo.rs`](https://github.com/ramate-io/maybraid/blob/cursor/mythical-chain-material-f9d3/maybraid/world/materials/chain/src/halo.rs)
spawns a child with the **same `Mesh3d` handle** and an additive
`ChainMaterial`, but it does **not** copy `SkinnedMesh`. On a static
placeholder that is fine; on A04’s armature the shell would remain in bind
pose.

Contract:

1. Halo child is a **child of the skinned mesh entity**, `Transform::IDENTITY`.
2. Halo copies `Mesh3d` **and** `SkinnedMesh { inverse_bindposes, joints }`
   (same handle, same joint entity list). Bevy skins both with one armature.
3. Halo material is a unique handle (`set_halo_pass(true)`); it copies
   authored scalars, `impact`, and `link_damage` from the parent each frame
   (already done).
4. Expansion stays in the vertex stage (normal * metres). No second mesh.
5. `sync_chain_halos` must run after A04 inserts `SkinnedMesh`, and must
   **refresh** the child’s `SkinnedMesh` if the parent’s joints change.

### Damage events → link ids

[`apply_hits`](../../../../damage/src/apply.rs) is the **only**
`Health::take_damage` on this path. [`DamageApplied`](../../../../damage/src/lib.rs)
is a report (`applied` = HP already lost, `remaining` = pool after that
apply). A downstream router that calls `take_damage(applied)` would injure
the chain twice.

Overlapping capsules must be collapsed **before** `DamageSystems::Apply`,
not after. `Flight::note_contact` keys on the collider entity, so one
bolt that crosses two chain capsules already emits two
[`ProjectileContact`](../../../../projectiles/src/lib.rs)s and two `Hit`s;
`apply_hits` then remounts both onto the same chain-root `Health` and
mutates twice. Deduping `DamageApplied` cannot undo that.

Same-frame `(source, chain_root)` is the wrong key: two bolts or two
[`AttackInstanceId`](../../../../attacks/src/instance.rs)s from one
attacker are distinct interactions and must both apply.

```text
ProjectileContact / attack hazard / melee (capsule entity)
        │
        │  #1127 coalescer — DamageSystems::Collect, BEFORE Apply
        │  Remount capsule → chain root via Health::entity_or_parent
        │  Dedupe overlapping colliders by interaction identity:
        │    projectile: ProjectileContact.projectile (the Flight entity)
        │    attack:     AttackInstanceId (hazards already fold candidates
        │                onto resolved_target() + the instance hit_clock)
        │    other:      a producer-owned interaction id, never
        │                (source, chain_root)
        │  One interaction × one chain → one Hit
        │  Distinct flights / instances in the same frame → distinct Hits
        ▼
Hit { target: remounted chain_root (or one chosen capsule), point, amount }
        │  apply_hits → take_damage ONCE → DamageApplied + optional Died
        ▼
DamageApplied { target: chain_root, point, applied, remaining }
        │  chain presentation router (new, #1127) — no Health mutation
        │  ChainLinkId on the chosen capsule, else nearest joint to `point`
        ▼
apply_chain_hit(owned_material, link_id, …)   // #1145 visual only
        │
        ├─ remaining > 0  → restamp MaterialRef intact/fade if global look
        └─ remaining == 0 / Died(chain) → emit ChainBroken once
                              detach sockets, retire capsules + joints
                              #1121 records Ward unlock
```

Boss-body hits never enter this coalescer. Attack hazards that already
resolve through [`HitCandidate::resolved_target`](../../../../attacks/src/hazard.rs)
keep that fold; the chain coalescer is required for the projectile
`Collect` path, where `contacts_to_hits` still emits one `Hit` per
collider.

### Open policy questions ([#1125](https://github.com/ramate-io/maybraid/issues/1125)–[#1127](https://github.com/ramate-io/maybraid/issues/1127))

| # | Question | Recommendation |
|---|---|---|
| C1 | Linked-chain (rigid per-link weights) vs continuous rope blend? | **Working recommendation: rigid per-link weights** (one joint owns each link). Matches color-R link ids, capsules, and `link_damage[i]`. Blend only a 1-ring seam if a gap appears. Continuous-energy look is the *shader* (#1145 veins), not the skin. **Open question for Liam:** [#1125](https://github.com/ramate-io/maybraid/issues/1125) also requires smoothly deforming bent poses without seam breaks or stretching discontinuities. Confirm this rigid recommendation against that mesh requirement before A04 starts; do not treat it as signed. |
| C2 | Restraint: gameplay kinematics, constraints, or physical joints? | **Prescribed kinematics** for the first slice: endpoints from column socket + character socket; in-between joints from a catenary / taut lerp; capsules follow joints. Runtime armature ≠ physics. Avian joints later if the prescribed pose fights terrain. |
| C3 | Break look: dissolve, localized gap, or severed pieces? | **Whole-chain fade** via existing `fade` / `intact` restamp + retire colliders / joints. One-mesh skinning does not sever. Debris / two-body split is a later VFX issue. |
| C4 | UV1.x metres vs keep #1145’s UV0.y `[0,1]`? | **Adopt UV1.x metres** as the stable flow coordinate. Ask the #1145 owner to migrate pulse/impact off UV0.y. UV0 stays around-link + link-local for the atlas. This is the one #1145 integration this proposal asks for. |
| C5 | Chain `Health` vs a dedicated durability component? | **Reuse `Health` on the chain root.** `Died` on the chain is the one-shot break. #1121 distinguishes `Died(chain)` (Ward) from `Died(boss)` (Enraged). No second HP crate. |

### First slice

Playground-first; no encounter host required.

1. **#1125 (Liam):** generate mesh + joints + bindposes + the attribute
   table above; cache by config; tests for straight / bent / min / longer;
   no Bikanwarda rig dependency.
2. **#1145 integration (material owner):** UV1.x flow + halo copies
   `SkinnedMesh`. Can land against the placeholder mesh (write UV1.x =
   `uv0.y * rest_length_m`).
3. **#1126 (Liam):** prescribed pose from two anchors; capsules on joints;
   `ChainLinkId`; teardown removes restraint + colliders.
4. **#1127:** coalesce overlapping chain-capsule contacts **before**
   `apply_hits`, keyed on projectile / `AttackInstanceId` (not attacker);
   `apply_hits` mutates chain `Health` once; `DamageApplied` / `Died` drive
   `apply_chain_hit`, `ChainBroken`, and fade + retire. Report unlock
   through #1121, do not implement inventory here.

---

## Downstream: what each decision unblocks

These lanes stay **optional** for Self. They unblock Bikanwarda / material
work only.

| Decision | Unblocks |
|---|---|
| **B1** `Id::Bytes` slug | [#1121](https://github.com/ramate-io/maybraid/issues/1121) can key killed/freed / Enraged vs Ward without coupling to AABB edits or stream-out. |
| **B2** dedicated `EncounterNeighborhood` + `Scripted<T>` | [#1117](https://github.com/ramate-io/maybraid/issues/1117) can author one row near Discovery `home` and stream it without a boss registry or Barking occupancy. |
| **B3** encounters crate, no `lubikans` dep | Self stays shippable; #1121 is the first crate that may touch both. |
| **B4** placeholder recipe | #1117 / #1121 gameplay can proceed before Liam’s #1133 GLBs. |
| **B5** no flatten in #1116 | #1117 owns the spawn-anchor / pad question. |
| **C1** rigid link weights (working rec.; Liam reviews vs #1125 smooth deform) | #1145 `link_damage[link_id]` and #1127 capsule → link id stay 1:1 **if** rigid weights stand. A blended rope still needs a stable link-id attribute. |
| **C2** prescribed pose | #1126 playground + #1117 socket attach without waiting on Avian joints. |
| **C3** whole-chain fade | #1145 `fade`/`intact` is the break visual; #1127 does not invent debris. |
| **C4** UV1.x metres + halo `SkinnedMesh` | **#1145 integration**: pulse/impact survive A04 length changes; halo follows the armature. |
| **C5** chain-root `Health` | #1127 / #1121 can tell `Died(chain)` from `Died(boss)` with the existing damage crate. |

---

## Decisions that need human judgment

Please confirm or replace the **Recommendation** column. Implementation
should not start on #1116 / #1125 / #1126 until B1–B2 and C2–C4 are signed,
and until Liam has reviewed **C1** against the #1125 smooth-deform mesh
requirement.

1. **B1** — stable `Id::Bytes` slug vs Richmond-style cell id.
2. **B2** — dedicated encounter channel vs ride `MobNeighborhood`.
3. **C1** — **open for Liam:** rigid per-link skin (this note’s working
   recommendation) vs blended rope, judged against #1125’s smoothly
   deforming bent-pose requirement. Do not change the recommendation here;
   do not start A04 until Liam signs or replaces it.
4. **C2** — prescribed kinematics vs physical joints for v0.1.4.
5. **C3** — fade-to-release vs severed pieces.
6. **C4** — commit UV1.x metres as the #1145 flow coordinate (material-owner
   change) vs keep UV0.y `[0,1]` and drop the metres channel.

Ownership reminder: Liam implements #1116, #1125, #1126, and final #1133
GLBs. This document is the review surface, not an assignment grab.
