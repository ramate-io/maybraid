# Grassy Mounds

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** ground-cover grove (see section 3.4.3 in the main RFC).


Grassy Mounds are discrete rounded ground-cover features based on the [Sparse Boulder](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#31-sparse-boulders) placement pattern, but shaded and embedded as vegetation rather than exposed rock.

Good for meadow irregularity, mossy hummocks, pasture texture, wetland edges, and soft terrain breakup.

```rust
pub enum GrassyMoundsCell {
    Mound(Bucket {
        weight: 1.0,
        item: Mound {
            placement: SparseBoulderLike {
                cell_size: 5.0,
                object_scale: 0.60,
                embed_depth: Deep,
            },
            shader: GroundCoverShader,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
        },
    }),
}

impl CellGrove for GrassyMounds {
    type Cell = GrassyMoundsCell;

    const CELL_SIZE_RANGE: Range<f32> = 5.0..6.0;
    const DENSITY_RANGE: Range<f32> = 0.25..0.55;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.85;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.35;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.40;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.02..0.06;
}
```

**Construction**

* Use Sparse Boulder-style cell placement.
* Use internal cells around `5m`.
* Set mound size to roughly `60%` of the cell.
* Use rounded SDF forms rather than angular rock forms.
* Embed more deeply than sparse boulders, so the mound reads as terrain growth, not an object sitting on top.
* Use ground-cover or leaf shaders rather than stone shaders.
* Collision may be enabled when mound height materially affects traversal.

**Placement**

```rust
let cell_size = 5.0;
let mound_radius = 0.60 * cell_size;
let position = sparse_boulder_position(cell, seed);

if !contains(parent_cell, position) {
    return None;
}

spawn_mound(
    position,
    radius = mound_radius,
    embed_depth = 0.25 * mound_radius,
    shader = GroundCoverShader,
);
```
