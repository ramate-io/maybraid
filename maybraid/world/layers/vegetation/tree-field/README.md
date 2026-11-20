# GPU-driven tree canopies

Working tree renderer: trunk, major limbs, a bounded branch skeleton, and a
layered foliage canopy generated on GPU from compact plant descriptors. Follows
the same requirements as
[#1637](https://github.com/ramate-io/maybraid/issues/1637) and
[#1638](https://github.com/ramate-io/maybraid/issues/1638), scaled up to trees.

Reuses the WESL library and grass/bush patch / select / indirect infrastructure.
Wood and foliage are distinct draw buckets. Tree recipes stay in this crate.

## Architecture

```text
CPU TreePatch (seed, storybook recipe, surface samples)
        │ compact Plant 96 B / Branch 80 B / Leaf 64 B  (no per-branch ECS)
        ▼
persistent GPU storage
        │ compute select_wood  — frustum, LOD, topology-preserving prune
        │ compute select_foliage — reattach to major_parent when twigs drop
        │ compute finalize — draw_indexed_indirect × 6
        ▼
Opaque3d tubes (near/mid/far) + layered canopy cards
```

| Piece | Role |
| --- | --- |
| `TreeRecipe::from_storybook` | Thin adapter onto Temperate Lower Massives storybook ranges |
| `TreePatch` / `TreeField` | CPU ownership and persistent descriptor buffers |
| `TreeSelector` | CPU mirror of `select_wood.wesl` / `select_foliage.wesl` |
| `wood.wesl` / `foliage.wesl` | Vertex-generated tubes and layered leaves importing shared WESL |

Far LOD keeps trunk + major limbs plus aggregated canopy cards so the crown
silhouette and height remain. `LodThresholds::tree_default()` is farther than
bush card distance.

## Tests

```bash
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-shaders --release --locked
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-tree-field --release --locked
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-tree-field-playground --release --locked
```

## Playground

```bash
nix develop --command cargo run -p vegetation-tree-field-playground --release --locked
```

Captures:

```bash
MAYBRAID_CAPTURE_DIR=maybraid/world/layers/vegetation/tree-field-playground/captures \
  nix develop --command cargo run -p vegetation-tree-field-playground --release --locked
```
