# 3.4.7.5: Rolling Oaks

Rolling Oaks is a low-density upper-canopy grove for open oak country. It uses common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md) variants at `5m-20m` and rare [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) variants at the same scale.

```rust
pub enum RollingOaksCell {
    RollingBraidOak(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.08..0.72,
            steepness: 0.0..0.48,
        },
        item: BraidOak {
            height: 5.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[oak_bark..dry_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[olive_green..fresh_green], [deep_green..yellow_green]],
        },
    }),
    RareRollingStorybook(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.08..0.68,
            steepness: 0.0..0.54,
        },
        item: StorybookTree {
            height: 5.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[broadleaf_bark..dry_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[olive_green..light_green], [deep_green..yellow_green]],
        },
    }),
}

impl CellGrove for RollingOaks {
    type Cell = RollingOaksCell;

    const CELL_SIZE_RANGE: Range<f32> = 14.0..30.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.34;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.036;
}
```

## Construction

* Use low-density placement, roughly `8%-24%`.
* Let Braid Oak dominate; keep Storybook variants rare.
* Favor rolling hill and open woodland constraints over wet valley constraints.
* Pair with grass, low scrub, and scattered understory rather than dense lower canopy.
