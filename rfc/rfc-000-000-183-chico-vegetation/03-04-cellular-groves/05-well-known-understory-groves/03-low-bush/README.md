# 3.4.5.3: Low Bush

Low Bush is a moderate-density understory grove using the [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/12-common-high-bush/README.md) construction scaled into the `50cm–1.5m` range. It represents low shrubs, young bushes, and woody filler vegetation that sit above ground cover but below full understory mass.

It should feel structured but not dominant: individual bushes are visible as rounded or vase-like forms, with enough spacing for ground cover, tufts, stones, roots, or exposed soil to remain legible between them.

Good for woodland edges, chaparral transition, sparse jungle understory, old fields, garden margins, path edges, and low filler around larger trees.

```rust
pub enum LowBushCell {
    GreenLowBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.65,
        },
        item: CommonHighBush {
            height: 0.50..1.20,
            shoot_count: 5..=8,
            projection_count: Low,
            branching: 1..=2,
            leaf_radius: 0.04..0.08,
            stick_palette_mix: [
                [shrub_bark..green_brown],
                [dark_bark..gray_brown],
            ],
            canopy_palette_mix: [
                [dark_green..light_green],
                [scrub_green..fresh_green],
                [deep_green..yellow_green],
            ],
        },
    }),
    DryLowBush(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.65,
        },
        item: CommonHighBush {
            height: 0.50..1.10,
            shoot_count: 4..=7,
            projection_count: Low,
            branching: 1..=2,
            leaf_radius: 0.03..0.07,
            stick_palette_mix: [
                [dry_bark..tan_brown],
                [gray_brown..straw_brown],
            ],
            canopy_palette_mix: [
                [dry_green..straw_brown],
                [olive_green..tan_green],
                [pale_green..dry_yellow_green],
            ],
        },
    }),
    LeafyLowBush(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.35,
            steepness: 0.0..0.35,
        },
        item: CommonHighBush {
            height: 0.80..1.50,
            shoot_count: 7..=10,
            projection_count: Moderate,
            branching: 2..=3,
            leaf_radius: 0.05..0.10,
            stick_palette_mix: [
                [shrub_bark..dark_bark],
                [green_brown..wet_brown],
            ],
            canopy_palette_mix: [
                [lush_green..bright_green],
                [deep_green..fresh_green],
                [blue_green..light_green],
            ],
        },
    }),
    FloweringLowBush(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.35,
            steepness: 0.0..0.65,
        },
        item: CommonHighBush {
            height: 0.60..1.20,
            shoot_count: 5..=8,
            projection_count: Low,
            branching: 1..=2,
            leaf_radius: 0.04..0.08,
            stick_palette_mix: [
                [shrub_bark..tan_brown],
                [green_brown..dark_bark],
            ],
            canopy_palette_mix: [
                [green..light_green],
                [flower_pink..leaf_green],
                [flower_white..fresh_green],
            ],
        },
    }),
    RedStemLowBush(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.45,
            steepness: 0.0..0.70,
        },
        item: CommonHighBush {
            height: 0.60..1.30,
            shoot_count: 5..=9,
            projection_count: Low,
            branching: 1..=3,
            leaf_radius: 0.04..0.09,
            stick_palette_mix: [
                [red_bark..copper_red],
                [burgundy_brown..dark_bark],
            ],
            canopy_palette_mix: [
                [dark_green..fresh_green],
                [yellow_green..light_green],
                [flower_pink..leaf_green],
            ],
        },
    }),
}

impl CellGrove for LowBush {
    type Cell = LowBushCell;

    const CELL_SIZE_RANGE: Range<f32> = 2.5..6.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.45;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.03..0.12;
}
```

## Construction

* Use moderate placement, roughly `18%–45%`.
* Use Common High Bush forms around `50cm–1.5m`.
* Keep shoot count and projection count lower than full bush constructions.
* Favor rounded, low silhouettes that leave gaps between neighboring bushes.
* Use multiple palette ranges, so repeated bushes do not read as cloned shrubs.
* Include rare red-stem variants for visible color punctuation in otherwise low shrub layers.
* Use deterministic yaw, scale, shoot count, and branch density sampling.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use where understory should feel woody but still low and permeable.
* Pair with [Floor Scrub](../../03-well-known-ground-cover-groves/04-floor-scrub/README.md), [Bush Scrub](../../04-well-known-tufts-groves/03-bush-scrub/README.md), exposed soil, stones, and tree bases.
* Works well as filler around path edges, open woodland, garden-like terrain, and young regrowth.
* Keep density below hedge-like coverage unless the region needs an intentionally blocked edge.
* Avoid using alone for dense understory; it should read as low shrub structure within a larger vegetation mix.
