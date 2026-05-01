# 3.4.7.17: Leeward

Leeward is a moderate-density upper-canopy grove using common [Temperate Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-15-temperate-conifer/README.md) and [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) variants at `10m-20m`.

It blends the sparse, fronded texture of Temperate Conifer with rounded broadleaf Storybook forms, suiting mild slopes sheltered from stronger weather.

```rust
pub enum LeewardCell {
    ShelteredTemperateConifer(Bucket {
        weight: 0.8,
        placement_constraints: PlacementConstraints {
            elevation: 0.10..0.72,
            steepness: 0.0..0.50,
        },
        item: TemperateConifer {
            height: 10.0..18.0,
            canopy_density: Moderate,
            stick_palette_mix: [[temperate_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[soft_green..deep_green], [blue_green..fresh_green]],
        },
    }),
    WindbreakTemperateConifer(Bucket {
        weight: 0.6,
        placement_constraints: PlacementConstraints {
            elevation: 0.16..0.78,
            steepness: 0.0..0.66,
        },
        item: TemperateConifer {
            height: 16.0..24.0,
            canopy_density: Sparse,
            stick_palette_mix: [[wind_barked..temperate_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[soft_green..blue_green], [deep_green..fresh_green]],
        },
    }),
    RoundedLeewardStorybook(Bucket {
        weight: 0.8,
        placement_constraints: PlacementConstraints {
            elevation: 0.08..0.70,
            steepness: 0.0..0.52,
        },
        item: StorybookTree {
            height: 10.0..18.0,
            canopy_density: Dense,
            stick_palette_mix: [[broadleaf_bark..brown_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[broadleaf_green..light_green], [deep_green..yellow_green]],
        },
    }),
    HighLeewardStorybook(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.12..0.68,
            steepness: 0.0..0.58,
        },
        item: StorybookTree {
            height: 16.0..24.0,
            canopy_density: Moderate,
            stick_palette_mix: [[broadleaf_bark..brown_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[broadleaf_green..light_green], [deep_green..yellow_green]],
        },
    }),
}

impl CellGrove for Leeward {
    type Cell = LeewardCell;

    const CELL_SIZE_RANGE: Range<f32> = 12.0..26.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.38;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.042;
}
```

## Construction

* Use moderate-density placement, roughly `18%-38%`.
* Keep Temperate Conifer and Storybook Tree evenly common overall.
* Use sheltered conifers and rounded Storybook Trees as the core, with taller windbreak and high-crown variants where the lee slope opens.
* Use sheltered-slope, mild-climate constraints rather than alpine or riparian constraints.
* Pair with understory that can handle partial shade and wind protection.
