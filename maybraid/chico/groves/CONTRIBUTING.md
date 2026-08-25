# Contributing (`chico-groves`)

Woody High/Medium plants should be **posed kit instances** of a quantized, merged construction — not a nest of per-stick / per-ball LOD hosts. Canonical woody grove: [`src/orchard.rs`](src/orchard.rs). Also flattened: [`src/goettingen_follow.rs`](src/goettingen_follow.rs), [`src/rolling_oaks.rs`](src/rolling_oaks.rs), [`src/vineyard.rs`](src/vineyard.rs), [`src/storytellers.rs`](src/storytellers.rs), [`src/high_bush.rs`](src/high_bush.rs), [`src/low_bush.rs`](src/low_bush.rs), [`src/spotty_bushes.rs`](src/spotty_bushes.rs), [`src/riverine_green.rs`](src/riverine_green.rs), [`src/forlorn_savanna.rs`](src/forlorn_savanna.rs), [`src/bush_scrub.rs`](src/bush_scrub.rs), [`src/riparian_general.rs`](src/riparian_general.rs), [`src/alpine.rs`](src/alpine.rs), [`src/christmas_taiga.rs`](src/christmas_taiga.rs), [`src/conifer_sapling.rs`](src/conifer_sapling.rs), [`src/arid_conifer_sapling.rs`](src/arid_conifer_sapling.rs), [`src/conifer_massives.rs`](src/conifer_massives.rs), [`src/dryland.rs`](src/dryland.rs), [`src/leeward.rs`](src/leeward.rs), [`src/jerrys_chaparral.rs`](src/jerrys_chaparral.rs), [`src/riparian_mix.rs`](src/riparian_mix.rs), [`src/levantine_scrub.rs`](src/levantine_scrub.rs), [`src/date_grove.rs`](src/date_grove.rs), [`src/palm_shade.rs`](src/palm_shade.rs), [`src/strange_oasis.rs`](src/strange_oasis.rs), [`src/tropical_undergrowth.rs`](src/tropical_undergrowth.rs), [`src/wandering_acacia.rs`](src/wandering_acacia.rs), [`src/trade_winds.rs`](src/trade_winds.rs), [`src/shamanhome.rs`](src/shamanhome.rs), [`src/tropical_thicket.rs`](src/tropical_thicket.rs), [`src/jungle_massives.rs`](src/jungle_massives.rs), [`src/jungle_lower_massives.rs`](src/jungle_lower_massives.rs), [`src/unending_jungle.rs`](src/unending_jungle.rs), [`src/temperate_massives.rs`](src/temperate_massives.rs), [`src/temperate_lower_massives.rs`](src/temperate_lower_massives.rs). Canonical tuft grove: [`src/monster_grass.rs`](src/monster_grass.rs). Plant-type merge / `unit_from_num` lives in [`chico-sbs-trees` CONTRIBUTING](../sbs-trees/CONTRIBUTING.md).

Comfortable unique visible meshes: a few hundred. `tree_variants` / `patch_variants` default **100**. Grove **tile** bands stay independent of the plant’s own structural factors (Orchard High / Medium / Low is `2 / 5 / 12`, not Storybook’s `10 / 30 / 50`). Plant High can be wide: [`ChicoLeafMaterial`](../shaders/src/chico_leaf_material.wgsl) cheapens far cheap-ball cheese — see [sbs-trees CONTRIBUTING §4](../sbs-trees/CONTRIBUTING.md#4-widening-high-is-a-shader-problem-then-a-factor). Large-tree groves extend the **tile Medium** band — see [Tile bands for large trees](#tile-bands-for-large-trees).

## Plant type first

If the tree or tuft still emits one node per stick/ball and has no `into_unit_from_num`, do that in `chico-sbs-trees` before changing the grove. Quantizing a unique-mesh construction only caps grow noise; it does not cap GPU uploads.

## Quantize at grow

1. Add `tree_variants: u32` (woody) or `patch_variants: u32` (tuft) on params, default `100`.
2. At grow, map placement → archetype with [`patch_variant_index`](src/grove/vc_tuft.rs) (stable hash of world XZ).
3. Key construction noise with [`variant_noise`](src/grove/vc_tuft.rs) so the same index rebuilds identically.
4. Call `params.into_unit_from_num(variant)` (or `unit_from_num`). Keep **palette / leaf color** on placement-keyed noise, not on the variant — color is an instance material, not a mesh key. Braid Oak groves instance [`BraidOakTree::unit_from_num`](../sbs-trees/src/braid_oak_tree.rs) and put sampled cell height on `Placement` only — do not remix grove SBS projection or stalk onto the mesh.
5. Put world size on [`Placement`](../vegetation-components/src/placed.rs):

   `Placement::new(position, 0.0).with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)))`

6. Store `Arc<YourTree>` when `T` is large (Orchard: `Arc<StorybookTree>`). Begin/drain must not clone the grown chain per chunk.

Orchard `grow_plant` is the woody template. Tuft groves use [`unit_plant_from_params`](src/grove/vc_tuft.rs).

## Flatten High / Medium

1. Compose plants with [`nest_flattened_plant_chunk`](src/grove/vc_compose.rs), not `nest_placed_plant_chunk`. Flattened hosts wrap `FlattenedComponentsOnly<PlacedVegetation<T>>` and spawn posed kits only.
2. Lazy `SceneChunk` for the plant list (`SceneChunk::lazy(n, n, …)` yielding one flattened chunk per plant). Begin must not box every `scene_with_level` up front. See Orchard `nest_plant_chunks`.
3. Feed that list through [`woody_grove_scene_chunks`](src/grove/vc_compose.rs) (or the tuft equivalent).
4. Low / UltraLow still author canopy proxies (`canopy_proxy_site`, `ULTRA_LOW_CANOPY_BIN_METERS`) but emit them through [`flattened_canopy_proxy_chunks`](../vegetation-components/src/lib.rs) (one cheap-ball collection kit, no per-plant `FoliageNode` hosts). `woody_grove_scene_chunks` does this on the Low branch.
5. Flattened kits already charge [`FLATTENED_KIT_CHUNK_WEIGHT`](../vegetation-components/src/lib.rs). Do not treat a GLB instance as weight 1.

## Tile bands for large trees

Woody High and Medium both nest plant hosts; Low swaps the tile to canopy proxies. The plant→blob edge is `medium_factor × tile_radius`, not High. Default preview / vast tiles stay near [`DEFAULT_GROVE_EXTENT_XZ`](src/grove/extent.rs) (100 m, radius 50 m) unless placement cells or an RFC patch need more room.

When typical trees are large, **extend Medium** (and raise Low so UltraLow does not slam in right after). Do not grow grove extent to fake a longer band — bigger tiles make one pop cover more ground and the center-based probe sits farther from edge trees. Do not copy plant factors onto the tile.

Rule of thumb: keep kits until a typical plant on that grove would itself be past plant Medium, including a tree on the far edge of the tile:

`medium_factor × tile_radius ≳ plant_medium_factor × (typical_height / 2) + tile_radius`

Worked example: [Storytellers](src/storytellers.rs) `5 / 20 / 30` on a 100 m tile keeps plants to 1 km (`20 × 50`) and proxies through 1.5 km. Orchard `2 / 5 / 12` is fine for small fruit trees; massives and tall storybook / oak groves should look more like Storytellers than Orchard.

## Playground host

Register **the flattened wrapper** in [`vegetation_lod.rs`](../sbs-trees-playground/src/vegetation_lod.rs):

```rust
avian_host!(app, FlattenedComponentsOnly<PlacedVegetation<Arc<YourTree>>>);
```

Keep `ComponentsOnly<PlacedVegetation<YourTree>>` only if another grove still nests fine-phase hosts. Do not add a second produce plugin per region channel.

## Tests

- Same cell positions + `tree_variants = 4` (or similar) produce repeated archetypes (`tree_variants_quantize_archetypes` / `patch_variants_quantize_archetypes`).
- High/Medium nest one flattened host per plant, not one host per kit node.
- Low / UltraLow `scene_chunks` emit one flattened cheap-ball collection, not one `FoliageNode` host per plant.

## What not to do

- Grow a full unique `StorybookTree` / `TuftPatch` per cell and only merge at emit — unique meshes stay unique.
- Flatten without `unit_from_num` — each plant is still a new merge key.
- Put world height into the unit mesh and identity placement scale.
- Stamp `LodLazyPending` on empty UltraLow stubs or host shells; kit BSN helpers already stamp SceneRef + MaterialRef roots.
- Grow grove extent to keep large trees as kits — extend tile Medium instead.
