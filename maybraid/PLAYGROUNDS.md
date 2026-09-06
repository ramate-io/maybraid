# Playgrounds

A playground is a **single-layer** developer app next to the crate it inspects (`richmond-buildings-playground`, `durham-terrain-models-playground`, …). Assembled world — Durham terrain, streamed forest, urbanization, character — lives in [`maybraid-world`](world/) and runs as [`maybraid-world-playground`](world/playground/).

Do not keep a second assembled-world app. Parameters and streaming knobs belong on `WorldPlugin` / `maybraid-world`, not on a parallel vegetation-on-terrain binary.

## Retiring

When a playground (or other throwaway app) is no longer worth maintaining:

1. Record it under [Retired](#retired) **before** deleting files: crate path, the last commit that still contained it (`git rev-parse HEAD` at retirement), and a short description of what it did.
2. Remove the crate from the workspace (and any `cargo run -p` docs). Point remaining callers at the replacement.
3. Restore later with `git checkout <commit> -- <path>` and re-add the workspace member.

If the crate is also a **library** used by world or another host, retire the **binary** and record that. Keep the library until those types move into a non-playground crate.

## Retired

Last commit that still contained these trees: [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06).

### `maybraid/chico/sbs-trees-playground`

- **Last commit:** [`4e2e1a35657840bb0ace01eb2e8d68bfecbb3e90`](https://github.com/ramate-io/maybraid/commit/4e2e1a35657840bb0ace01eb2e8d68bfecbb3e90)
- **Did:** Isolated `/show` plants and tiled groves (`vast-orchards`, `monster-grass-plains`, …), then a leftover flat-ground `/forest` streamer after `ForestPlugin` moved into `chico-forests`. By this commit the `/show` catalog was already gone.
- **Replacement:** [`maybraid-world-playground`](world/playground/) for streamed forest on Durham. Isolated plant `/show` is not hosted anywhere; restore this crate if that catalog is needed again.

### `maybraid/chico/vegetation-on-terrain-playground` (binary)

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Small Durham fine-grid patch for iterating Chico groves on real ground. `/grove <kind>` tiled one grove type; `/forest` streamed the same generate/present/cull path as SBS, grown on Durham height. Character / free-look, canopy bump-outs, mesh stats.
- **Replacement:** [`maybraid-world-playground`](world/playground/) (`cargo run -p maybraid-world-playground`). The crate remains as `VegetationHostPlugin` (character / camera) plus `VegetationPlugin<S>` (forest + bump-outs) for `maybraid-world` (`OnTerrain<DurhamHeight>`, get-only) and Richmond developments-on-terrain. World terrain is `TerrainPlugin<Durham>`. World does not register urbanization until pad bake and host spawn stay on a per-frame budget ([#720](https://github.com/ramate-io/maybraid/issues/720)).

### `playgrounds/terrain` (`terrain-playground`)

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Early SDF marching-cubes terrain LOD chunk viewer (`engine` + `procedures/terrain`).

### `playgrounds/objects` (`objects-playground`)

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Mesh inspection for walls, trees, and groves on the pre-Chico `vegetation-sdf` / `procedures/buildings` stack.

### `playgrounds/skill-map` (`skill-map-playground`)

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Skill-map demo (fireballs on pink squares, lock on blue) reusing the objects playground stack.

### `playgrounds/pathfinding` (`pathfinding-playground`)

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** 2D local pathfinding: a red agent steers around a wall toward the cursor via `procedures/intelligence`.

### `demos/naturescapes` (`naturescapes-demo`)

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Navigable naturescapes: procedures terrain SDF, Durham water shaders, and `vegetation-sdf`. Ancestor of Durham / Chico world composition.

### `engine`

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Shared Bevy chunk manager, marching cubes, and outline/leaf shaders used only by the trees above.

### `procedures/terrain`, `procedures/vegetation`, `procedures/buildings`, `procedures/skill-map`, `procedures/intelligence`

- **Last commit:** [`af1f65fe59f0b6d01061ca2716119c9df7c68f06`](https://github.com/ramate-io/maybraid/commit/af1f65fe59f0b6d01061ca2716119c9df7c68f06)
- **Did:** Pre-Maybraid generation: 2.5D height-oracle terrain SDF ([RFC-105](../rfc/rfc-000-000-105-procedural-terrain/README.md)), ball-stick `vegetation-sdf`, early building meshes, skill-map noise, and local pathfinding. Replaced by Durham, Chico, Richmond, and `maybraid/intelligence`.
- **Moved:** `comproc` (guillotine + noise) now lives at [`maybraid/procedural/comproc`](procedural/comproc/). SDF / ball-stick modules that only served the old stack were dropped.
