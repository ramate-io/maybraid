# 3.4.7.16: Wandering Acacia

Wandering Acacia is a very-low-density upper-canopy grove for dry open country. It uses common acacia-impression [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-12-common-high-bush/README.md) variants at `5m-15m` and dry [Sope's Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-06-sope-s-banyan/README.md) variants at `5m-20m`.

```rust
pub enum WanderingAcaciaCell {
    WanderingHighBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.08..0.78,
            steepness: 0.0..0.66,
        },
        item: CommonHighBush {
            height: 5.0..15.0,
            canopy_density: Sparse,
            stick_palette_mix: [[acacia_bark..red_brown], [tan_bark..gray_brown]],
            canopy_palette_mix: [[dusty_green..olive_green], [yellow_green..dry_green]],
        },
    }),
    DryWanderingSopesBanyan(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.06..0.62,
            steepness: 0.0..0.58,
        },
        item: SopesBanyan {
            height: 5.0..20.0,
            canopy_density: Sparse,
            descender_frequency: Sparse,
            stick_palette_mix: [[dry_banyan_bark..tan_bark], [red_brown..dark_bark]],
            canopy_palette_mix: [[olive_green..dusty_green], [deep_green..dry_green]],
        },
    }),
}

impl CellGrove for WanderingAcacia {
    type Cell = WanderingAcaciaCell;

    const CELL_SIZE_RANGE: Range<f32> = 22.0..52.0;
    const DENSITY_RANGE: Range<f32> = 0.03..0.12;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.14..0.44;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.005..0.026;
}
```

## Construction

* Use very-low-density placement, roughly `3%-12%`.
* Keep High Bush more common than dry Sope's Banyan.
* Use acacia-like flat, sparse crowns and dry palettes.
* Let the grove wander across open dry terrain rather than forming a closed canopy.
