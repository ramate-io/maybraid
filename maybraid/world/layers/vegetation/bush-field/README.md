# GPU-driven small bushes

Working shrub renderer: connected tapered stems plus attached foliage, generated
on GPU from compact plant descriptors. Tracks
[#1638](https://github.com/ramate-io/maybraid/issues/1638).

Reuses the WESL library and grass patch / select / indirect infrastructure from
[#1637](https://github.com/ramate-io/maybraid/issues/1637). Wood and foliage are
distinct draw buckets, not a grass reskin.

## Architecture

```text
CPU BushPatch (seed, recipe, surface samples)
        │ compact Plant / Branch / Leaf descriptors (no per-branch ECS)
        ▼
persistent GPU storage  ──►  compute select wood + foliage (frustum, LOD, prune)
                            ──►  compute finalize (draw_indexed_indirect × 6)
                            ──►  Opaque3d tubes (wood) + folded leaves (foliage)
```

| Piece | Role |
| --- | --- |
| `BushPatch` | CPU ownership. Compact topology from a Bush Scrub small-bush recipe. |
| `BushField` | Persistent plant / branch / leaf buffers. Generation bumps on change. |
| `BushSelector` | CPU mirror of `select_wood.wesl` / `select_foliage.wesl`. |
| `wood.wesl` / `foliage.wesl` | Vertex-generated tubes and folded leaves importing shared WESL. |

Parent/child junctions stay welded through wind: children evaluate in the
deformed parent `attachment_frame`. Pruning a branch also drops or reattaches
its dependents. Distant bushes keep a crown silhouette until projected height
is below `LodThresholds::bush_default()`.

## Tests

```bash
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-shaders --release --locked
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-bush-field --release --locked
PRE_COMMIT_DISABLE=true nix develop --command cargo test -p vegetation-bush-field-playground --release --locked
```

## Playground

```bash
nix develop --command cargo run -p vegetation-bush-field-playground --release --locked
```

Captures:

```bash
MAYBRAID_CAPTURE_DIR=maybraid/world/layers/vegetation/bush-field-playground/captures \
  nix develop --command cargo run -p vegetation-bush-field-playground --release --locked
```
