# 3.4.5.8: Levantine Scrub

Levantine Scrub is a dry Mediterranean understory grove using [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-07-rory-s-head-trained/README.md), small [Vase Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-03-vase-tree/README.md), [High Bush](../03-04-05-04-high-bush/README.md), small [Penmarch Torch](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-04-penmarch-torch/README.md), and [Simpleman's Hedge](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-16-simpleman-s-hedge/README.md) constructions.

It represents a dry, cultivated-to-wild scrub layer: trained horizontal crowns and small vase forms mix with woody bushes, torch-shaped ornamental shrubs, and occasional hedge-like bands.

Good for Mediterranean hillsides, olive-grove edges, dry gardens, old terraces, ruins, rocky valleys, and warm scrubland transitions.

```rust
pub enum LevantineScrubCell {
    DryRoryHeadTrained(Bucket {
        weight: 1.2,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.70,
            steepness: 0.0..0.70,
        },
        item: RoryHeadTrained {
            height: 1.20..3.00,
            stalk_radius: 0.030,
            canopy_spread: 0.80..2.20,
            canopy_density: Sparse,
            stick_palette_mix: [
                [dry_bark..gray_brown],
                [vine_bark..olive_brown],
            ],
            canopy_palette_mix: [
                [olive_green..dry_green],
                [silver_green..pale_green],
                [dark_green..yellow_green],
            ],
        },
    }),
    SmallVaseTree(Bucket {
        weight: 0.70,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.65,
            steepness: 0.0..0.52,
        },
        item: VaseTree {
            height: 1.20..3.00,
            stalk_radius: 0.030,
            canopy_spread: 0.70..1.80,
            canopy_density: Sparse..Moderate,
            stick_palette_mix: [
                [ornamental_bark..gray_brown],
                [dry_bark..tan_brown],
            ],
            canopy_palette_mix: [
                [olive_green..light_green],
                [dry_green..flower_white],
                [dark_green..silver_green],
            ],
        },
    }),
    DryHighBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.72,
            steepness: 0.0..0.65,
        },
        item: CommonHighBush {
            height: 1.00..2.50,
            shoot_count: 7..=11,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.05..0.11,
            stick_palette_mix: [
                [dry_bark..tan_brown],
                [gray_brown..straw_brown],
            ],
            canopy_palette_mix: [
                [olive_green..dry_green],
                [scrub_green..tan_green],
                [pale_green..yellow_green],
            ],
        },
    }),
    SmallPenmarchTorch(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.10..0.70,
            steepness: 0.0..0.64,
        },
        item: PenmarchTorch {
            height: 1.40..3.20,
            stalk_radius: 0.030,
            canopy_spread: 0.50..1.30,
            canopy_density: Sparse..Moderate,
            stick_palette_mix: [
                [dry_bark..dark_bark],
                [ornamental_bark..gray_brown],
            ],
            canopy_palette_mix: [
                [dark_green..olive_green],
                [dry_green..light_green],
                [flower_yellow..pale_green],
            ],
        },
    }),
    RedOliveTorch(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.10..0.68,
            steepness: 0.0..0.60,
        },
        item: PenmarchTorch {
            height: 1.60..3.40,
            stalk_radius: 0.030,
            canopy_spread: 0.55..1.35,
            canopy_density: Sparse,
            stick_palette_mix: [
                [copper_red..orange_bark],
                [red_brown..dark_bark],
            ],
            canopy_palette_mix: [
                [olive_green..silver_green],
                [flower_yellow..light_green],
                [dark_green..pale_green],
            ],
        },
    }),
    ScrubHedge(Bucket {
        weight: 0.50,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.65,
            steepness: 0.0..0.35,
        },
        item: SimplemansHedge {
            height: 0.80..1.60,
            width: 0.70..1.80,
            density: Moderate,
            palette_mix: [
                [hedge_green..olive_green],
                [dry_green..pale_green],
                [flower_white..leaf_green],
            ],
        },
    }),
}

impl CellGrove for LevantineScrub {
    type Cell = LevantineScrubCell;

    const CELL_SIZE_RANGE: Range<f32> = 3.5..8.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.48;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.02..0.09;
}
```

## Construction

* Use sparse to moderate placement, roughly `18%–48%`.
* Use High Bush as the dominant scrub mass.
* Mix in Rory's Head-trained and small Vase Tree forms for cultivated, trained, or wind-shaped silhouettes.
* Add small Penmarch Torch variants sparingly for upright flame-like accents.
* Use rare red-stick torch variants as Mediterranean color punctuation.
* Use Simpleman's Hedge variants where the scrub should imply terrace edges, old gardens, or low barriers.
* Prefer dry Mediterranean palettes: olive, silver-green, pale green, yellow-green, tan-green, and occasional flowering highlights.
* Use deterministic yaw, scale, canopy spread, hedge width, and branch density sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where warm, dry scrub should feel halfway between wild brush and cultivated planting.
* Pair with dry grass, exposed soil, stone terraces, old paths, ruins, and sparse orchard or olive-like tree layers.
* Works well on hillsides and foothills where the scrub can tolerate moderate slope.
* Keep hedge variants occasional unless the region is explicitly gardened or terraced.
* Avoid dense tropical greens; the grove should read as dry, sun-exposed, and scrubby.
