# 3.4.4.3: Bush Scrub

Bush Scrub is a sparse tuft and small-bush grove using [Tuft](../../../03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md) and scaled-down [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/12-common-high-bush/README.md) constructions.

It represents low, irregular scrub with enough structure to suggest woody growth. Small bushes with low projection count may also read as saplings.

Good for arid regions, woodland edges, transitional understory, disturbed terrain, and sparse groves.

```rust
pub enum BushScrubCell {
    DryTuft(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.75,
        },
        item: Tuft {
            height: 0.25..0.45,
            width: 0.12..0.30,
            palette_mix: [dry_green..straw_brown],
        },
    }),
    GreenTuft(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.45,
        },
        item: Tuft {
            height: 0.25..0.50,
            width: 0.12..0.35,
            palette_mix: [dark_green..light_green],
        },
    }),
    SmallBush(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.65,
        },
        item: CommonHighBush {
            height: 0.35..0.80,
            shoot_count: 4..=7,
            projection_count: Low,
            branching: 1..=2,
            leaf_radius: 0.04..0.08,
            stick_palette_mix: [dry_bark..gray_brown],
            canopy_palette_mix: [scrub_green..dry_green],
        },
    }),
    SaplingBush(Bucket {
        weight: 0.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.65,
            steepness: 0.0..0.45,
        },
        item: CommonHighBush {
            height: 0.50..1.20,
            shoot_count: 3..=5,
            projection_count: VeryLow,
            branching: 1..=1,
            leaf_radius: 0.03..0.06,
            stick_palette_mix: [young_bark..green_brown],
            canopy_palette_mix: [young_green..light_green],
        },
    }),
}

impl CellGrove for BushScrub {
    type Cell = BushScrubCell;

    const CELL_SIZE_RANGE: Range<f32> = 2.0..5.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.30;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.40;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.03..0.10;
}
```

## Construction

* Use sparse placement, roughly `10%–30%`.
* Use tufts around `25cm–50cm`.
* Include several tuft varietals rather than a single color mix.
* Include small Common High Bush variants with:
  * reduced shoot count
  * low projection count
  * sparse branching
  * small leaf radius
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection

## Usage

* Use where ground cover should feel sparse but structured.
* Pair with [Floor Scrub](../../03-well-known-ground-cover-groves/04-floor-scrub/README.md) or exposed terrain.
* Works well in arid and semi-arid groves.
* Sapling-like variants can imply regrowth or young woodland without requiring full tree placement.
