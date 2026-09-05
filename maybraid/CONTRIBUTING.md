# Contributing

## Organization and Naming

There are a few key organization and naming rules that will help to track Maybraid development.

### Proper Noun Implementations 

Early implementations, particularly those following an RFC, will often have a name that is a proper noun. For example, [Crozon](./crozon/) and [Durham](./durham/) which are early implementations of characters and terrain types and systems respectively. 

General logic that is no longer tied to a proper-noun implementation belongs in a generally named crate — for example [`procedural/common`](procedural/common/) and [`procedural/comproc`](procedural/comproc/) (guillotine partitions, noise sampling).

### `-models` Crates

The term "model" and suffix `-models` is used to refer to a layer that defining the base behavior of a game object. Typically, this means taking a lower-order asset, such as tree from [`chico-sbs-trees`](./chico/sbs-trees/), and integrating it with standard game systems such as LOD, generation, and physics. Accordingly, models should typically define plugins that idempotently make available the needed systems for base behaviors.

**Plugin shape:** define one idempotent plugin **per model** (e.g. `TerrainPlugin` beside the terrain types), then compose those plugins at the **crate root** (e.g. `DurhamTerrainModelsPlugin` that only registers each model plugin). Apps should add the crate-root plugin once rather than wiring individual model plugins ad hoc—unless they intentionally need a subset.

Particularly bespoke systems, like player damage, movement, and inventory, are not necessarily considered parts of models until the underlying API is generalized. Before that point, they are expected to be implemented as separate systems acting on the types that the models define.

At the time of writing, building models refers to defining behavior with respect to...

- [`lod`](./lod/lib)
- [`generation`](./lod/lib/src/gen.rs)
- [Avian Physics](https://github.com/avianphysics/avian)

...and mostly consists of implementing the traits from [`generation`](./lod/lib/src/gen.rs) with the added colliders.

Sometimes, particularly during early development of a model, the game object may only be defined within the `*-models` crate. However, generally, things like the composition of a game object will be defined in another crate and then extended with the model. This pattern keeps the `*-models` crate focused on the behavior of the game object rather than its internal structure. For example, rather than defining the procedure to give all branches on a tree, the `*-models` crate implementation of the tree can simply focus on which branches are visible at a given LOD. Conversely, the crate implementing the tree does not have to worry about plugging into the generation dependency system from the start.

> [!IMPORTANT]
> Please update this section if increasing or different layers are consistently implemented at the `-models` level.

## Playgrounds

Iterate a **single layer** in a `*-playground` crate next to that layer. Compose Durham + forest + urbanization + character in [`world`](world/) (`cargo run -p maybraid-world-playground`). Do not add a second assembled-world binary.

Retiring a playground: [PLAYGROUNDS.md](PLAYGROUNDS.md) (record last commit and what it did under Retired, then delete).

## Chico vegetation trees (LOD)

Learnings from migrating ball-stick trees (Sope’s Banyan, Penmarch / Kamakura torch, Rory’s Head-trained) onto [`chico-vegetation-components`](./chico/vegetation-components/) + [`chico-sbs-trees`](./chico/sbs-trees/).

- **Naming:** `FooParams` = authoring / CLI; `Foo` = built instance from `params.build()` (grow the chain once). Prefer this over `*Instance` / `*Std` for new vegetation.
- **Grove preview params:** flatten [`GrovePreviewParams`](./chico/groves/src/grove/preview.rs) (`GroveFrontend`, extent, terrain, `tree_variants`, resolved placements). Grove-specific fields are only flags `build` still reads (`merge_collections`, tuft/bush palette-seed noise). Call surface: `Params::default().with_extent(e).build_on(&world)`.
- **Woody grove LOD:** authored HIGH / MEDIUM / LOW and canopy policy stay on the grove via [`WoodyGroveLod`](./chico/groves/src/grove/woody_lod.rs). Opening Orchard should still show `2 / 5 / 12` and `ordinary`. File submodules: `foo.rs` (recipe) + `foo/vc.rs` (clap/build/grow) + `foo/vc/tests.rs` / `foo/tests.rs`. Never `mod.rs`. Crate root re-exports `Foo` + `FooParams` only.
- **Presentation:** trees implement `VegetationComponents` and present via `FlattenedComponentsOnly<PlacedVegetation<Arc<T>>>` / `spawn_flattened_placed_vegetation`. Tuft groves without `LodScene` still use `ComponentsOnly` / `spawn_vegetation_components`.
- **Stick geometry:** `StickGeometry::{Segment,Trunk}` picks the kit triad under `vegetation/sticks/standard/` (`001_*` vs `trunk_001_*`) and the nested mesh-LOD extent policy. Trunk is geometry, not a second style.
- **Nested stick mesh LOD:** band on **radius/girth** for segments (`distance / radius`); trunks stay length-biased (max-axis extent). Useful default factors: High ≤ 10, Medium ≤ 25, Low ≤ 100; **UltraLow = empty scene** (do not collapse onto Low).
- **Structural (tree) LOD:** separate probe from stick/foliage hosts. Tall torches: characteristic radius `max(footprint, half-height)` so height does not dump you to Medium while still filling the view; torch defaults High / Medium / Low ≈ 3 / 15 / 24.
- **Silhouette sampling:** azimuth × height outer picks beat “every Nth” or global outer shells for vase / torch profiles. For sticks, sample the **outermost endpoint** (not the midpoint) — midpoints sit inward on steep limbs and lose the contest.
- **Share what is shared:** Penmarch and Kamakura share [`torch_tree`](./chico/sbs-trees/src/torch_tree.rs) stick + canopy emission. Rory can reuse stick thinning and structural factors but keep its own foliage **candidate** policy (joints vs selective BranchOut). Layered **mass proxies** are optional per tree and LOD — tune mid vs upper placement; some trees want none (e.g. Rory).
- **Foliage kits:** `cheap_ball` for dense banded samples; `layered_ball` for proxies / fuller near masses. High can still band (not emit every terminal) to cut near-duplicates.
- **Fronds:** authored \(Y \in [0,1]\), \(X \in [-0.1,0.1]\), \(Z\) negligible. Prefer [`FoliageGeometry::FrondCollection`](./chico/vegetation-components/src/foliage/geometry.rs) (polyline-partition style: many leaf kits, **one** [`FoliageNode`](./chico/vegetation-components/src/foliage/node.rs) / [`FoliageLodProbe`](./chico/vegetation-components/src/foliage/probe.rs)). Authored connectivity is [`FrondRun`](./chico/vegetation-components/src/foliage/collection/frond.rs) (base→tip chain); LOD thinning drops/collapses **runs**, never mid-chain segments. Presentation is [`CollectionPresent`](./chico/vegetation-components/src/foliage/present.rs) on the node: `Merge` (default, one `MultiSceneMerge`) or `Instance` (posed kits, same host). Bands: `distance / max_AABB_extent` with `FROND_COLLECTION_{HIGH,MEDIUM,LOW}_FACTOR` in that file — High = all runs, Medium ≈ half runs (full chains), Low ≈ quarter (collapse to chords), UltraLow = one marker.

Preview forest composition in [`maybraid-world-playground`](world/playground/) while walking LOD bands.

## Rust Style

Follow the top-level [Rust Style](../CONTRIBUTING.md#rust-style) guidance: prefer methods on structs/enums over free-floating helpers, and keep **"cell"** naming for LOD cellular generation—not for generic bounded rectangles in shared procedural code (`procedural-common`, etc.).