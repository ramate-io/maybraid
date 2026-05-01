# 3.4.7.13: Dryland

Dryland is a very-low-density upper-canopy grove for arid highlands and sparse dry forests. It uses common [Liam's Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-02-liam-s-conifer/README.md) and [Vase Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-03-vase-tree/README.md) variants at `10m-20m`.

```rust
pub enum DrylandCell {
    DrylandLiamsConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.22..0.86,
            steepness: 0.0..0.82,
        },
        item: LiamsConifer {
            height: 10.0..20.0,
            canopy_density: Sparse,
            stick_palette_mix: [[dry_conifer_bark..tan_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[sage_green..dusty_green], [deep_green..olive_green]],
        },
    }),
    DrylandVaseTree(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.18..0.78,
            steepness: 0.0..0.70,
        },
        item: VaseTree {
            height: 10.0..20.0,
            canopy_density: Sparse,
            stick_palette_mix: [[sun_baked_bark..tan_bark], [red_brown..gray_brown]],
            canopy_palette_mix: [[olive_green..dusty_green], [yellow_green..dry_green]],
        },
    }),
}

impl CellGrove for Dryland {
    type Cell = DrylandCell;

    const CELL_SIZE_RANGE: Range<f32> = 22.0..48.0;
    const DENSITY_RANGE: Range<f32> = 0.03..0.12;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.14..0.42;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.005..0.026;
}
```

## Construction

* Use very-low-density placement, roughly `3%-12%`.
* Keep Liam's Conifer and Vase Tree evenly common.
* Use dusty, sun-baked palettes and sparse canopies.
* Allow steep and exposed slopes, but keep the grove open.
