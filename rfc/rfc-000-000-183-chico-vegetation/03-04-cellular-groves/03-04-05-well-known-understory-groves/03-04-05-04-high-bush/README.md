# 3.4.5.4: High Bush

High Bush is a moderate-density understory grove using the [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-12-common-high-bush/README.md) construction scaled into the `1m–2.5m` range. It represents substantial shrub masses that sit between low bushes and small trees.

It should feel leafy, rounded, and body-height: bushes are tall enough to shape sightlines and local movement, but not tall enough to read as canopy vegetation. Compared to [Low Bush](../03-04-05-03-low-bush/README.md), High Bush uses stronger branching, larger silhouettes, and more visible foliage mass.

Good for dense woodland understory, riparian margins, jungle edges, old gardens, hedge-like natural barriers, and transition zones beneath larger trees.

```rust
pub enum HighBushCell {
    GreenHighBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.40,
            steepness: 0.0..0.32,
        },
        item: CommonHighBush {
            height: 1.00..2.20,
            shoot_count: 7..=10,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.06..0.12,
            stick_palette_mix: [
                [shrub_bark..green_brown],
                [dark_bark..gray_brown],
            ],
            canopy_palette_mix: [
                [deep_green..fresh_green],
                [dark_green..light_green],
                [blue_green..emerald_green],
            ],
        },
    }),
    DenseHighBush(Bucket {
        weight: 1.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.40,
            steepness: 0.0..0.32,
        },
        item: CommonHighBush {
            height: 1.40..2.50,
            shoot_count: 8..=12,
            projection_count: Moderate,
            branching: 3..=5,
            leaf_radius: 0.07..0.14,
            stick_palette_mix: [
                [dark_bark..wet_brown],
                [green_brown..shrub_bark],
            ],
            canopy_palette_mix: [
                [lush_green..bright_green],
                [wet_green..fresh_green],
                [deep_green..yellow_green],
            ],
        },
    }),
    DryHighBush(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.40,
            steepness: 0.0..0.32,
        },
        item: CommonHighBush {
            height: 1.00..2.00,
            shoot_count: 6..=9,
            projection_count: Moderate,
            branching: 2..=3,
            leaf_radius: 0.05..0.10,
            stick_palette_mix: [
                [dry_bark..tan_brown],
                [gray_brown..straw_brown],
            ],
            canopy_palette_mix: [
                [olive_green..dry_green],
                [tan_green..pale_green],
                [straw_brown..green],
            ],
        },
    }),
    BerryHighBush(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.40,
            steepness: 0.0..0.32,
        },
        item: CommonHighBush {
            height: 1.20..2.20,
            shoot_count: 7..=10,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.06..0.12,
            stick_palette_mix: [
                [shrub_bark..dark_bark],
                [green_brown..wet_brown],
            ],
            canopy_palette_mix: [
                [dark_green..leaf_green],
                [berry_red..deep_green],
                [berry_blue..fresh_green],
            ],
        },
    }),
    CopperCaneHighBush(Bucket {
        weight: 0.30,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.45,
            steepness: 0.0..0.58,
        },
        item: CommonHighBush {
            height: 1.20..2.50,
            shoot_count: 7..=11,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.06..0.12,
            stick_palette_mix: [
                [copper_red..orange_bark],
                [red_brown..dark_bark],
            ],
            canopy_palette_mix: [
                [deep_green..fresh_green],
                [yellow_green..light_green],
                [berry_red..leaf_green],
            ],
        },
    }),
}

impl CellGrove for HighBush {
    type Cell = HighBushCell;

    const CELL_SIZE_RANGE: Range<f32> = 3.5..8.0;
    const DENSITY_RANGE: Range<f32> = 0.16..0.42;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.03..0.10;
}
```

## Construction

* Use moderate placement, roughly `16%–42%`.
* Use Common High Bush forms around `1m–2.5m`.
* Use more shoot and branch complexity than Low Bush, but keep silhouettes distinct rather than merged into a hedge.
* Allow occasional dense variants to shape sightlines, especially near forest edges or paths.
* Include broadleaf, dry, lush, and fruiting palette variants where biome-appropriate.
* Include rare copper-cane variants where the understory needs a brighter bark accent.
* Use deterministic yaw, scale, shoot count, branch density, and foliage size sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where understory should form visible shrub masses without becoming full lower canopy.
* Pair with [Low Bush](../03-04-05-03-low-bush/README.md), Braid Grass, forest-floor ground cover, fallen logs, and tree bases.
* Works well as a natural soft barrier, path edge, forest transition, or dense background layer.
* Keep spacing open enough for player readability unless the design calls for occluding vegetation.
* Avoid using as small trees; it should remain shrub-like, rounded, and rooted near the ground.
