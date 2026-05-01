# 3.4.5.10: Riverine Green

Riverine Green is a sparse understory grove made from green [High Bush](../03-04-05-04-high-bush/README.md) and [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-12-common-high-bush/README.md) variants.

It represents scattered wet, leafy shrub growth along rivers, creeks, seeps, and damp lowland edges. The grove should feel greener and softer than Spotty Bushes, but still sparse enough to preserve open waterline readability.

Good for riverbanks, riparian shade, wet meadow edges, springs, damp hollows, pond margins, and green transition zones beneath taller riverside trees.

```rust
pub enum RiverineGreenCell {
    WetGreenBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.45,
            steepness: 0.0..0.42,
        },
        item: CommonHighBush {
            height: 1.00..2.20,
            shoot_count: 7..=11,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.06..0.13,
            stick_palette_mix: [
                [wet_bark..dark_bark],
                [green_brown..wet_brown],
            ],
            canopy_palette_mix: [
                [wet_green..fresh_green],
                [deep_green..light_green],
                [blue_green..emerald_green],
            ],
        },
    }),
    BrightBankBush(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.40,
            steepness: 0.0..0.65,
        },
        item: CommonHighBush {
            height: 0.80..1.70,
            shoot_count: 6..=10,
            projection_count: Moderate,
            branching: 2..=3,
            leaf_radius: 0.05..0.11,
            stick_palette_mix: [
                [young_bark..green_brown],
                [wet_brown..tan_bark],
            ],
            canopy_palette_mix: [
                [bright_green..light_green],
                [yellow_green..fresh_green],
                [lush_green..lime_green],
            ],
        },
    }),
    DeepShadeBush(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.50,
            steepness: 0.0..0.45,
        },
        item: CommonHighBush {
            height: 1.20..2.40,
            shoot_count: 8..=12,
            projection_count: Moderate,
            branching: 3..=5,
            leaf_radius: 0.07..0.14,
            stick_palette_mix: [
                [dark_bark..wet_brown],
                [green_brown..gray_brown],
            ],
            canopy_palette_mix: [
                [dark_green..deep_green],
                [blue_green..wet_green],
                [emerald_green..fresh_green],
            ],
        },
    }),
    PaleRiparianBush(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.42,
            steepness: 0.0..0.60,
        },
        item: CommonHighBush {
            height: 0.90..1.80,
            shoot_count: 6..=10,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.05..0.12,
            stick_palette_mix: [
                [wet_bark..gray_brown],
                [green_brown..tan_bark],
            ],
            canopy_palette_mix: [
                [pale_green..fresh_green],
                [silver_green..light_green],
                [yellow_green..wet_green],
            ],
        },
    }),
    RedTwigRiverBush(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.38,
            steepness: 0.0..0.55,
        },
        item: CommonHighBush {
            height: 0.90..1.90,
            shoot_count: 7..=11,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.05..0.12,
            stick_palette_mix: [
                [red_twig..copper_red],
                [wet_burgundy..dark_bark],
            ],
            canopy_palette_mix: [
                [wet_green..fresh_green],
                [bright_green..yellow_green],
                [silver_green..light_green],
            ],
        },
    }),
}

impl CellGrove for RiverineGreen {
    type Cell = RiverineGreenCell;

    const CELL_SIZE_RANGE: Range<f32> = 4.0..10.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.015..0.070;
}
```

## Construction

* Use sparse placement, roughly `8%–24%`.
* Use only green, wet, riparian High Bush variants; avoid dry scrub palettes.
* Allow rare red-twig variants for wetland edge color pop without breaking the green identity.
* Keep elevation constraints low, so these variants prefer valley floors, banks, and damp lowlands.
* Allow moderate slope, so shrubs can climb small stream banks and ravine edges.
* Use deterministic yaw, scale, shoot count, branch density, and leaf-size sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where water-adjacent vegetation needs sparse green punctuation.
* Pair with damp ground cover, reeds, [Braid Grass](../03-04-05-01-braid-grass/README.md), exposed mud, stones, and riparian tree layers.
* Works well along creeks, rivers, ponds, and shaded wet hollows.
* Keep coverage sparse enough that shorelines and path edges remain visible.
* Avoid dry, tan, or chaparral palettes; this grove should read as fresh and water-fed.
