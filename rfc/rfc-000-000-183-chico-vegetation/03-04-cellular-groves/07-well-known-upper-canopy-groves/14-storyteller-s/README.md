# 3.4.7.14: Storyteller's

Storyteller's is a moderate-density upper-canopy grove with colorful [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md) and [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md) variants at `10m-30m`.

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
    PinkLanternStorybook(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.62,
            steepness: 0.0..0.50,
        },
        item: StorybookTree {
            height: 8.0..18.0,
            canopy_density: Dense,
            stick_palette_mix: [[warm_bark..purple_brown], [red_brown..dark_bark]],
            canopy_palette_mix: [[hot_pink..rose_leaf], [fresh_green..pink_bloom]],
        },
    }),
    RedFestivalBraidOak(Bucket {
        weight: 0.30,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.60,
            steepness: 0.0..0.46,
        },
        item: BraidOak {
            height: 12.0..24.0,
            canopy_density: Moderate,
            stick_palette_mix: [[red_oak_bark..bright_red_bark], [copper_red..dark_bark]],
            canopy_palette_mix: [[red_leaf..copper_leaf], [gold_leaf..fresh_green]],
        },
    }),
    PurpleCrownStorybook(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.06..0.58,
            steepness: 0.0..0.54,
        },
        item: StorybookTree {
            height: 14.0..30.0,
            canopy_density: Sparse,
            stick_palette_mix: [[purple_brown..dark_bark], [red_brown..gray_brown]],
            canopy_palette_mix: [[violet_leaf..purple_leaf], [deep_green..rose_leaf]],
        },
    }),
    BlueMoonStorybook(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.08..0.64,
            steepness: 0.0..0.58,
        },
        item: StorybookTree {
            height: 12.0..22.0,
            canopy_density: Moderate,
            stick_palette_mix: [[blue_gray_bark..purple_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[blue_leaf..cyan_leaf], [deep_blue_green..fresh_green]],
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
* Use colorful stick and canopy palette mixes: reds, coppers, golds, rose leaves, blues, and bright greens.
* Include rare bright pink, red, purple, and blue variants with distinct heights and canopy densities, closer to the color-pop behavior of Wild Grass.
* Use this as a whimsical upper canopy rather than a biome-neutral forest.
