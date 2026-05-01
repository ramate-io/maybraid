# 3.4.7.14: Storyteller's

Storyteller's is a moderate-density upper-canopy grove with colorful [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) and [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md) variants at `10m-30m`.

```rust
pub enum StorytellersCell {
    ColorfulStorybook(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.72,
            steepness: 0.0..0.52,
        },
        item: StorybookTree {
            height: 10.0..30.0,
            canopy_density: Dense,
            stick_palette_mix: [[warm_bark..red_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[storybook_green..gold_green], [rose_leaf..fresh_green]],
        },
    }),
    ColorfulBraidOak(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.70,
            steepness: 0.0..0.48,
        },
        item: BraidOak {
            height: 10.0..30.0,
            canopy_density: Dense,
            stick_palette_mix: [[red_oak_bark..copper_red], [oak_bark..dark_bark]],
            canopy_palette_mix: [[deep_green..gold_green], [copper_leaf..fresh_green]],
        },
    }),
    BrightCanopyStorybook(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.06..0.66,
            steepness: 0.0..0.56,
        },
        item: StorybookTree {
            height: 10.0..26.0,
            canopy_density: Moderate,
            stick_palette_mix: [[purple_brown..red_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[gold_leaf..fresh_green], [rose_leaf..light_green]],
        },
    }),
}

impl CellGrove for Storytellers {
    type Cell = StorytellersCell;

    const CELL_SIZE_RANGE: Range<f32> = 14.0..30.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.38;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.042;
}
```

## Construction

* Use moderate-density placement, roughly `18%-38%`.
* Keep Storybook Tree and Braid Oak both common.
* Use colorful stick and canopy palette mixes: reds, coppers, golds, rose leaves, and bright greens.
* Use this as a whimsical upper canopy rather than a biome-neutral forest.
