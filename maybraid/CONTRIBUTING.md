# Contributing

## Organization and Naming

There are a few key organization and naming rules that will help to track Maybraid development.

### Layer names

A layer's **highest-order model** keeps a proper name: Durham (terrain), Richmond (urbanization), Chico (vegetation), and Barking (mobs). Those models live at [`world/layers/{category}/{name}`](./world/layers/) as the crates `durham`, `richmond`, `chico`, and `barking`.

Lower-order crates are named by what they do (`terrain-shaders`, `building-components`, `sbs-trees`, `mob-scenes`) and live under `world/layers/{category}`. Do not put a proper name on a shader, stamp, component, or playground crate.

Assets that belong to a named model may keep that name. RFC directory names stay.

Character crates live under [`world/layers/mobs/characters/`](./world/layers/mobs/characters/) and are named by what they do. Character UI menus live under [`menu/`](./menu/).

### `-models` Crates

The term "model" and suffix `-models` is used to refer to a layer that defining the base behavior of a game object. Typically, this means taking a lower-order asset, such as tree from [`sbs-trees`](./world/layers/vegetation/sbs-trees/), and integrating it with standard game systems such as LOD, generation, and physics. Accordingly, models should typically define plugins that idempotently make available the needed systems for base behaviors.

**Plugin shape:** define one idempotent resource plugin **per model** (e.g. `TerrainResourcesPlugin` beside the terrain types), then compose those plugins at the **crate root** (e.g. `DurhamTerrainModelsPlugin` that only registers each model plugin). World-facing fill lives on `Generate<Mode, OnTerrain<Durham>>` ([`install_durham_generation`](world/layers/terrain/durham/src/terrain/host.rs)); raw present is `Present<Mode, OnTerrain<Durham>>`. Model crates hold contracts and generation; if it draws, it lives in a presentation crate. Apps should add the crate-root or layer plugin once rather than wiring individual model plugins ad hoc—unless they intentionally need a subset.

Particularly bespoke systems, like player damage, movement, and inventory, are not necessarily considered parts of models until the underlying API is generalized. Before that point, they are expected to be implemented as separate systems acting on the types that the models define.

At the time of writing, building models refers to defining behavior with respect to...

- [`lod`](./lod/lib)
- [`generation`](./lod/lib/src/gen.rs)
- [Avian Physics](https://github.com/avianphysics/avian)

...and mostly consists of implementing the traits from [`generation`](./lod/lib/src/gen.rs) with the added colliders.

Sometimes, particularly during early development of a model, the game object may only be defined within the `*-models` crate. However, generally, things like the composition of a game object will be defined in another crate and then extended with the model. This pattern keeps the `*-models` crate focused on the behavior of the game object rather than its internal structure. For example, rather than defining the procedure to give all branches on a tree, the `*-models` crate implementation of the tree can simply focus on which branches are visible at a given LOD. Conversely, the crate implementing the tree does not have to worry about plugging into the generation dependency system from the start.

> [!IMPORTANT]
> Please update this section if increasing or different layers are consistently implemented at the `-models` level.

## Hierarchical generation (CSG)

World layers compose by **reusing `GenerationScheme`s**, not by re-deriving each other's discovery. To add a stage (a stamp band, a correction pass, a layer that sits on terrain):

1. **Write one `GenerationScheme` per generated type, beside that type, under its world layer.** `original_ids_for` says which ids originate in a region; `build_with_id` builds one id. Put shared discovery on the trait that owns the concept, not in free functions over `&mut S`. In Durham, `CellTiling::original_cell_ids_for` serves layout-gridded roots and `LeafAabbs::original_leaf_ids_for` serves leaves of a controller.
2. **Bound only on what you read directly.** If you read type `D`, require `S: GeneratingSpatialIndex<D>`. `D`'s own controllers, layouts, and configs are `D`'s bounds, not yours; they resolve at the concrete index. Use plain `SpatialIndex<D>` when you only read and never generate.
3. **Discover dependencies through the index.** Use `GeneratingSpatialIndex::<D>::for_each_origin(index, region, visit)`, or `original_ids_for` and then `get_one_or_generate` for each id. This keeps discovery lazy and in id order. Do not call `D::original_ids_for` or `<D as GenerationScheme<S>>::…` directly, and do not swap discovery for eager `get_or_generate_region`.
4. **Reuse existing ids.** If your type sits on an existing grid, delegate `original_ids_for` to that grid's root type. For example, `Terrain` and `Water` reuse [`PreWatershedTerrain`](world/layers/terrain/durham/src/terrain.rs)'s origin cells, and the watershed stages reuse `HydroComplexCell`'s.
5. **Seed universal singletons from `Bootstrap*` traits.** In Durham, use `universal_bootstrap_scheme!` instead of hand-writing a scheme for each config or layout.
6. **Use generic helpers when a scheme pulls a family of leaves.** Share the leaf shape through a trait (Durham's `StampLeaf`) instead of writing a macro for each type.
7. **Keep generation independent of the viewer.** Schemes never see a `LodRef` or camera pose. Presentation decides what content it needs (a distant forest needs selection and a canopy proxy; a nearby forest needs grove recipes and detailed geometry) and requests those ids. `descendants` is only for descendants that always accompany a node. If quality changes the generated value itself, make it part of cache identity (a separate type, id, or generation parameter), so two presentation paths never share an id while expecting different content.

The runtime follows the same rule: `LodGeneratePlugin<T, S>` needs only `S: GeneratingSpatialIndex<T>`. It still reads driver poses, but only to prioritize its queue.

## Chico vegetation trees (LOD)

Learnings from migrating ball-stick trees (Sope’s Banyan, Penmarch / Kamakura torch, Rory’s Head-trained) onto [`vegetation-components`](./world/layers/vegetation/components/) + [`sbs-trees`](./world/layers/vegetation/sbs-trees/).

- **Naming:** `FooParams` = authoring / CLI; `Foo` = built instance from `params.build()` (grow the chain once). Prefer this over `*Instance` / `*Std` for new vegetation.
- **Grove preview params:** flatten [`GrovePreviewParams`](./world/layers/vegetation/groves/src/grove/preview.rs) (`GroveFrontend`, extent, terrain, `tree_variants`, resolved placements). Grove-specific fields are only flags `build` still reads (`merge_collections`, tuft/bush palette-seed noise). Call surface: `Params::default().with_extent(e).build_on(&world)`.
- **Woody grove LOD:** authored HIGH / MEDIUM / LOW and canopy policy stay on the grove via [`WoodyGroveLod`](./world/layers/vegetation/groves/src/grove/woody_lod.rs). Opening Orchard should still show `2 / 5 / 12` and `ordinary`. File submodules: `foo.rs` (recipe) + `foo/vc.rs` (clap/build/grow) + `foo/vc/tests.rs` / `foo/tests.rs`. Never `mod.rs`. Crate root re-exports `Foo` + `FooParams` only.
- **Presentation:** trees implement `VegetationComponents` and present via `FlattenedComponentsOnly<PlacedVegetation<Arc<T>>>` / `spawn_flattened_placed_vegetation` (isolated `/show` and grove plants). Tuft groves without `LodScene` still use `ComponentsOnly` / `spawn_vegetation_components`.
- **Stick geometry:** `StickGeometry::{Segment,Trunk}` picks the kit triad under `vegetation/sticks/standard/` (`001_*` vs `trunk_001_*`) and the nested mesh-LOD extent policy. Trunk is geometry, not a second style.
- **Nested stick mesh LOD:** band on **radius/girth** for segments (`distance / radius`); trunks stay length-biased (max-axis extent). Useful default factors: High ≤ 10, Medium ≤ 25, Low ≤ 100; **UltraLow = empty scene** (do not collapse onto Low).
- **Structural (tree) LOD:** separate probe from stick/foliage hosts. Tall torches: characteristic radius `max(footprint, half-height)` so height does not dump you to Medium while still filling the view; torch defaults High / Medium / Low ≈ 3 / 15 / 24.
- **Silhouette sampling:** azimuth × height outer picks beat “every Nth” or global outer shells for vase / torch profiles. For sticks, sample the **outermost endpoint** (not the midpoint) — midpoints sit inward on steep limbs and lose the contest.
- **Share what is shared:** Penmarch and Kamakura share [`torch_tree`](./world/layers/vegetation/sbs-trees/src/torch_tree.rs) stick + canopy emission. Rory can reuse stick thinning and structural factors but keep its own foliage **candidate** policy (joints vs selective BranchOut). Layered **mass proxies** are optional per tree and LOD — tune mid vs upper placement; some trees want none (e.g. Rory).
- **Foliage kits:** `cheap_ball` for dense banded samples; `layered_ball` for proxies / fuller near masses. High can still band (not emit every terminal) to cut near-duplicates.
- **Fronds:** authored \(Y \in [0,1]\), \(X \in [-0.1,0.1]\), \(Z\) negligible. Prefer [`FoliageGeometry::FrondCollection`](./world/layers/vegetation/components/src/foliage/geometry.rs) (polyline-partition style: many leaf kits, **one** [`FoliageNode`](./world/layers/vegetation/components/src/foliage/node.rs) / [`FoliageLodProbe`](./world/layers/vegetation/components/src/foliage/probe.rs)). Authored connectivity is [`FrondRun`](./world/layers/vegetation/components/src/foliage/collection/frond.rs) (base→tip chain); LOD thinning drops/collapses **runs**, never mid-chain segments. Presentation is [`CollectionPresent`](./world/layers/vegetation/components/src/foliage/present.rs) on the node: `Merge` (default, one `MultiSceneMerge`) or `Instance` (posed kits, same host). Bands: `distance / max_AABB_extent` with `FROND_COLLECTION_{HIGH,MEDIUM,LOW}_FACTOR` in that file — High = all runs, Medium ≈ half runs (full chains), Low ≈ quarter (collapse to chords), UltraLow = one marker.

Preview isolated plants by restoring [`chico-sbs-trees-playground`](PLAYGROUNDS.md#retired) if that `/show` catalog is needed. Streamed forest on Durham lives in [`maybraid-world-playground`](world/playground/).

## Playgrounds

Iterate a **single layer** in a `*-playground` crate next to that layer. Compose Durham + forest + urbanization + character in [`world`](world/) (`cargo run -p maybraid-world-playground`). Do not add a second assembled-world binary.

Retiring a playground: [PLAYGROUNDS.md](PLAYGROUNDS.md) (record last commit and what it did under Retired, then delete).

## Rust Style

Follow the top-level [Rust Style](../CONTRIBUTING.md#rust-style) guidance: prefer methods on structs/enums over free-floating helpers, and keep **"cell"** naming for LOD cellular generation—not for generic bounded rectangles in shared procedural code (`procedural-common`, etc.).