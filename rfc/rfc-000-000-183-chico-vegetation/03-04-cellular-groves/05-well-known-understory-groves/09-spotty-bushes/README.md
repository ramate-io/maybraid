# 3.4.5.9: Spotty Bushes

Spotty Bushes is a very sparse understory grove made from varied [High Bush](../04-high-bush/README.md) and [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/12-common-high-bush/README.md) forms.

It represents isolated shrub punctuation rather than a continuous understory layer. Each placement should read as a distinct bush or small cluster with plenty of terrain, grass, rocks, or ground cover visible between placements.

Good for open woodland, meadow edges, sparse hillsides, dry groves, garden margins, regrowth patches, and transitional terrain that needs occasional shrub structure without becoming brush.

```rust
pub enum SpottyBushesCell {
    GreenSpotBush(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.45,
            steepness: 0.0..0.48,
        },
        item: CommonHighBush {
            height: 1.00..2.10,
            shoot_count: 6..=10,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.05..0.12,
            stick_palette_mix: [
                [shrub_bark..green_brown],
                [dark_bark..gray_brown],
            ],
            canopy_palette_mix: [
                [deep_green..fresh_green],
                [dark_green..light_green],
                [scrub_green..yellow_green],
            ],
        },
    }),
    DrySpotBush(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.70,
            steepness: 0.0..0.55,
        },
        item: CommonHighBush {
            height: 0.80..1.80,
            shoot_count: 5..=9,
            projection_count: Low..Moderate,
            branching: 1..=3,
            leaf_radius: 0.04..0.09,
            stick_palette_mix: [
                [dry_bark..tan_brown],
                [gray_brown..straw_brown],
            ],
            canopy_palette_mix: [
                [dry_green..olive_green],
                [tan_green..pale_green],
                [straw_brown..green],
            ],
        },
    }),
    DenseSpotBush(Bucket {
        weight: 0.60,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.40,
            steepness: 0.0..0.42,
        },
        item: CommonHighBush {
            height: 1.40..2.50,
            shoot_count: 8..=12,
            projection_count: Moderate,
            branching: 3..=5,
            leaf_radius: 0.07..0.14,
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
    FloweringSpotBush(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.00..0.65,
            steepness: 0.0..0.38,
        },
        item: CommonHighBush {
            height: 0.90..1.80,
            shoot_count: 6..=10,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.05..0.11,
            stick_palette_mix: [
                [shrub_bark..tan_brown],
                [green_brown..dark_bark],
            ],
            canopy_palette_mix: [
                [dark_green..leaf_green],
                [flower_white..fresh_green],
                [flower_pink..light_green],
            ],
        },
    }),
}

impl CellGrove for SpottyBushes {
    type Cell = SpottyBushesCell;

    const CELL_SIZE_RANGE: Range<f32> = 5.0..12.0;
    const DENSITY_RANGE: Range<f32> = 0.04..0.16;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.28;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.015..0.060;
}
```

## Construction

* Use very sparse placement, roughly `4%–16%`.
* Use varied High Bush and Common High Bush forms, not a single repeated shrub.
* Keep each placement visually separate; do not allow the grove to read as a hedge or thicket.
* Mix green, dry, dense, and occasional flowering variants.
* Let dry variants tolerate more slope than dense or flowering variants.
* Use deterministic yaw, scale, shoot count, branch density, and foliage-size sampling.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use where an area needs occasional shrub landmarks without becoming brushy.
* Pair with open grass, sparse ground cover, rocks, tree bases, and meadow-edge vegetation.
* Works well as low-frequency filler across broad terrain.
* Keep density low enough that individual bushes remain countable at medium distance.
* Avoid using where the design needs continuous occlusion or path-shaping vegetation.
