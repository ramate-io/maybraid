# 3.4.7.4: Riparian General

Riparian General is a moderate-density upper-canopy grove for mixed river corridors. It uses common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md) and [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) variants at `5m-15m`, with rare [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-12-common-high-bush/README.md) forms stretched into small tree scale.

```rust
pub enum RiparianGeneralCell {
    RiparianBraidOak(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.42,
            steepness: 0.0..0.36,
        },
        item: BraidOak {
            height: 5.0..15.0,
            canopy_density: Moderate,
            stick_palette_mix: [[wet_oak_bark..dark_bark], [moss_bark..gray_brown]],
            canopy_palette_mix: [[river_green..fresh_green], [deep_green..yellow_green]],
        },
    }),
    RiparianStorybook(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.44,
        },
        item: StorybookTree {
            height: 5.0..15.0,
            canopy_density: Moderate,
            stick_palette_mix: [[broadleaf_bark..wet_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[river_green..light_green], [deep_green..fresh_green]],
        },
    }),
    RareRiparianHighBush(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.38,
            steepness: 0.0..0.52,
        },
        item: CommonHighBush {
            height: 5.0..15.0,
            canopy_density: Sparse,
            stick_palette_mix: [[willow_bark..wet_brown], [red_brown..gray_brown]],
            canopy_palette_mix: [[fresh_green..yellow_green], [river_green..light_green]],
        },
    }),
}

impl CellGrove for RiparianGeneral {
    type Cell = RiparianGeneralCell;

    const CELL_SIZE_RANGE: Range<f32> = 10.0..22.0;
    const DENSITY_RANGE: Range<f32> = 0.20..0.42;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.28;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.012..0.050;
}
```

## Construction

* Use moderate-density placement, roughly `20%-42%`.
* Keep Braid Oak and Storybook Tree common and even.
* Use rare High Bush variants as willow-like or shrubby tree accents.
* Bias constraints toward low elevation, flat floodplain, and stream-adjacent terrain.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.
