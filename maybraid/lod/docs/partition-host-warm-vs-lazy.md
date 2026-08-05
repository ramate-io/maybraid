# Partition hosts: warm H/M/L vs lazy LodRef fine-pass

Playground / Wizard’s Tower experiment (release, M4 Pro). Numbers are
`system_commands` apply times from the playground timing layer unless noted.

## Setups

| Setup | Partition `scene_with_lod` | Fine-pass |
| --- | --- | --- |
| **Warm** | `warm_content_host_hsl` (High+Medium+Low content roots) | Probe writes level; visibility flips |
| **Static bake** | Default `scene_with_level(current)` only (no host) | None for partitions |
| **Lazy** | `LodSceneHost` + `PartitionNode` + one current root | `add_fine_pass_for::<PartitionNode>` (eager fulfill + cull) |

Tower Medium remains one scene-chunk (not per-floor) in these runs.

## Appear cost (Medium / exterior stand-up)

| Setup | Approx. apply | Hosts after stand-up |
| --- | --- | --- |
| Warm | ~8 ms (debug ~85 ms) | ~220 |
| Static bake | ~2.8 ms | 1 (tower only) |
| Lazy | ~3.4 ms first forest, then ~4.4 ms at tower Medium | ~220 |

Lazy is lighter **per level root** than warm (one mesh tier, not three). That matches
expectation.

## The abrasive pattern (lazy)

With lazy hosts, the same structural work shows up as **two separate expensive
frames** rather than one warmer hitch:

1. **First forest** — when the building first presents partitions under the
   active tower band (here ~3.4 ms, `hosts≈220`).
2. **Approach thrash** — many frames of `update` → `spawn_requests=N` →
   `cull despawned=N` as partition bands flip (4–10 hosts per frame in the
   captured approach). Individually small, but continuous spawn/despawn.
3. **Tower Medium** — another ~4.4 ms apply when the Medium exterior root
   materializes and stands up its own partition-host forest under that root.

So the user-facing feel can be **worse than a single ~8 ms warm hitch**: you pay
a smaller bill twice (and nibble in between) instead of once.

Warm’s trade: higher one-shot appear (all three mesh tiers), cheaper band flips
afterward (visibility only).

Lazy’s trade: cheaper per root, but **re-pay insert cost** when a new parent
level root embeds a new host forest, plus **ongoing fulfill/cull** while moving.

Static bake is cheapest to appear but does **not** update partition mesh LOD
with the camera — not a fair functional baseline.

## Reading the lazy log (sketch)

```
apply≈3.4ms  sync hosts=220          # first partition forest
… update changed=N / spawn_requests=N / cull despawned=N …  # approach
chunk begin Medium …
apply≈4.4ms  complete                # Medium exterior + nested hosts
sync hosts=219
```

## Implications

- Do not treat “lazy is faster on Medium apply” as a win without watching the
  approach path and any **second** parent-band stand-up.
- If the dominant event is “building band appears,” warm (or warm-adjacent-only)
  may still feel smoother than lazy nested hosts under a one-shot parent spawn.
- If the dominant event is long-lived hosts with rare band changes, lazy (and
  later chunked fulfill) is the better fit.
- Nested hosts under a chunked/lazy **tower** still multiply cost: Medium is one
  tower chunk that eagerly embeds N partition hosts.

## Related

- [`scene-chunks.md`](scene-chunks.md) — tower-level incremental fulfill
- `PartitionNode::scene_with_lod` — lazy host wiring
- Playground: `add_fine_pass_for::<PartitionNode>()` (eager), tower still on
  `add_fine_pass_chunk_full_for::<WizardsTower>()`
