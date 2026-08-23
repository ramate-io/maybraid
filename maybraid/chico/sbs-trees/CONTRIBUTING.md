# Contributing (`chico-sbs-trees`)

Merge + quantization is how a construction stays cheap when a grove instances it many times. Canonical tree: [`src/storybook_tree.rs`](src/storybook_tree.rs). Also done: [`src/vase_tree.rs`](src/vase_tree.rs), [`src/jungle_storybook_tree.rs`](src/jungle_storybook_tree.rs), [`src/braid_oak_tree.rs`](src/braid_oak_tree.rs), [`src/rorys_head_trained.rs`](src/rorys_head_trained.rs), [`src/penmarch_torch.rs`](src/penmarch_torch.rs), [`src/kamakura_torch.rs`](src/kamakura_torch.rs). Canonical tuft: [`src/tuft_patch.rs`](src/tuft_patch.rs). Grove wiring lives in [`chico-groves` CONTRIBUTING](../groves/CONTRIBUTING.md).

Comfortable unique visible meshes: a few hundred. `tree_variants` / `patch_variants` default **100**.

## Why two steps

Without quantization, every plant is a unique grown mesh (`SceneRef` / `MultiSceneMerge` cache miss per placement). Without merge, each stick and cheap ball is its own spawn even after quantization.

Quantize first (shared unit archetype), then merge (one stick collection + one cheap-ball collection per level).

## Tree construction

Do this on the **plant type** (`StorybookTree`, `VaseTree`, …), not only in the grove.

### 1. Unit archetype

Add `unit_from_num` / `into_unit_from_num` on params and the built type (see `StorybookTreeParams::into_unit_from_num`):

1. Record pre-normalize world size (tree height, or tuft `max(extent, blade_length)`).
2. Scale authored lengths/radii so the construction is **unit** height or footprint.
3. Key layout / canopy / tuft **seed by `num`**, not by world position.
4. Return `(unit_params, world_size)`. World size goes on plant [`Placement`](../vegetation-components/src/placed.rs) scale in the grove.

Same `num` must rebuild the same chain. Different `num` must differ. Test both.

Do **not** bake world height into the mesh. Grove placement scale (`placed.scale * world_size`) is the instance size.

### 2. Merge on emit

In `VegetationComponents` for High (and Medium if it still uses kits):

```rust
fn merge_sticks(nodes: Vec<StickNode>) -> Vec<StickNode> {
    StickNode::merge_standard(nodes).into_iter().collect()
}

fn merge_foliage(nodes: Vec<FoliageNode>) -> Vec<FoliageNode> {
    let (cheap, rest) = /* split CheapBall / CheapBallCollection vs other */;
    let mut out = rest;
    if let Some(merged) = FoliageNode::merge_cheap_balls(cheap) {
        out.insert(0, merged);
    }
    out
}
```

[`StickNode::merge_standard`](../vegetation-components/src/sticks/node.rs) and [`FoliageNode::merge_cheap_balls`](../vegetation-components/src/foliage/node.rs) become one [`MultiSceneMerge`](../../scene-ref) per collection. Merge already packs kit-local positions into vertex **COLOR** so [`ChicoLeafMaterial`](../shaders) breakup still works after the bake.

Leave layered / frond / procedural fallbacks as separate nodes. Low canopy proxies do not need this merge.

### 3. LOD bands stay local

Pass `AzimuthHeightBands` at the `*_banded` call site. Do not call `torch_tree::stick_nodes_high` (or similar) if that hides another construction’s cell counts. Declare High / Medium / Low band constants on **this** module.

### 4. Tests

- `unit_from_num(n)` is unit-sized and deterministic.
- `into_unit_from_num` returns the pre-normalize world size.
- High emit is one stick collection and one cheap-ball collection (Storybook: `high_emits_merged_stick_and_cheap_ball_collections`).

## Grove construction

Quantization is wasted if the grove still grows a unique `T` per cell. See [groves CONTRIBUTING](../groves/CONTRIBUTING.md): `tree_variants` / `patch_variant_index` / `into_unit_from_num` / `nest_flattened_plant_chunk`.
