# 3.4.7.6: Forlorn Savanna

Forlorn Savanna is a low-density upper-canopy grove for sparse, wind-shaped dry landscapes. It uses common [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md) variants at `5m-30m`, common acacia-impression [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/12-common-high-bush/README.md) variants at `5m-10m`, and rare [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md) variants at `10m-20m`.

```rust
pub enum ForlornSavannaCell {
    SavannaRory(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.10..0.72,
            steepness: 0.0..0.58,
        },
        item: RoryHeadTrained {
            height: 5.0..30.0,
            canopy_density: Sparse,
            canopy_spread: 3.0..12.0,
            stick_palette_mix: [[weathered_bark..dark_bark], [red_brown..gray_brown]],
            canopy_palette_mix: [[olive_green..dry_green], [yellow_green..dusty_green]],
        },
    }),
    AcaciaHighBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.08..0.78,
            steepness: 0.0..0.64,
        },
        item: CommonHighBush {
            height: 5.0..10.0,
            canopy_density: Sparse,
            stick_palette_mix: [[acacia_bark..red_brown], [tan_bark..gray_brown]],
            canopy_palette_mix: [[dusty_green..olive_green], [yellow_green..dry_green]],
        },
    }),
    RareSavannaStorybook(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.12..0.68,
            steepness: 0.0..0.50,
        },
        item: StorybookTree {
            height: 10.0..20.0,
            canopy_density: Sparse,
            stick_palette_mix: [[dry_brown..dark_bark], [gray_brown..tan_bark]],
            canopy_palette_mix: [[olive_green..yellow_green], [dusty_green..light_green]],
        },
    }),
}

impl CellGrove for ForlornSavanna {
    type Cell = ForlornSavannaCell;

    const CELL_SIZE_RANGE: Range<f32> = 18.0..42.0;
    const DENSITY_RANGE: Range<f32> = 0.06..0.20;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.12..0.38;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.006..0.030;
}
```

## Construction

* Use low-density placement, roughly `6%-20%`.
* Keep Rory's Head-trained and acacia-like High Bush forms common.
* Use Storybook variants rarely, so the grove stays open and forlorn.
* Use dry bark, olive, yellow-green, and dusty canopy palettes.
