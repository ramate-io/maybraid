# 3.4.4.3: Bush Scrub

Bush Scrub is a sparse tuft and small-bush grove using [Tuft](../../../03-01-stalk-and-ball-stick-trees/03-01-02-ball-components/03-01-02-06-tufts/README.md) and scaled-down [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-12-common-high-bush/README.md) constructions.

It represents low, irregular scrub with enough structure to suggest woody growth. Small bushes with low projection count may also read as saplings.

Good for arid regions, woodland edges, transitional understory, disturbed terrain, and sparse groves.

```rust
pub enum BushScrubCell {
    DryTuft(Bucket {
        weight: 2.0,
        item: Tuft {
            height: 0.25..0.45,
            width: 0.12..0.30,
            palette_mix: [dry_green..straw_brown],
        },
    }),
    GreenTuft(Bucket {
        weight: 1.5,
        item: Tuft {
            height: 0.25..0.50,
            width: 0.12..0.35,
            palette_mix: [dark_green..light_green],
        },
    }),
    SmallBush(Bucket {
        weight: 1.0,
        item: CommonHighBush {
            height: 0.35..0.80,
            shoot_count: 4..=7,
            projection_count: Low,
            branching: 1..=2,
            leaf_radius: 0.04..0.08,
            palette_mix: [scrub_green..dry_green],
        },
    }),
    SaplingBush(Bucket {
        weight: 0.5,
        item: CommonHighBush {
            height: 0.50..1.20,
            shoot_count: 3..=5,
            projection_count: VeryLow,
            branching: 1..=1,
            leaf_radius: 0.03..0.06,
            palette_mix: [young_green..light_green],
        },
    }),
}

impl CellGrove for BushScrub {
    type Cell = BushScrubCell;

    const CELL_SIZE_RANGE: Range<f32> = 2.0..5.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.30;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.85;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.45;

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
* Use deterministic yaw, scale, and varietal selection.

```rust
let p = cell_origin + offset;
let normal = terrain_normal(p);

match selected {
    BushScrubCell::DryTuft(_) | BushScrubCell::GreenTuft(_) => {
        spawn_tuft(
            position = p,
            direction = normalize(mix(Vec3::Y, normal, 0.35)),
            scale = sampled_height,
            yaw = TAU * noise(seed, ROTATION_SALT),
        );
    }
    BushScrubCell::SmallBush(_) | BushScrubCell::SaplingBush(_) => {
        spawn_common_high_bush(
            position = p,
            height = sampled_height,
            shoot_count = sampled_shoot_count,
            branching = sampled_branching,
        );
    }
}
```

## Usage

* Use where ground cover should feel sparse but structured.
* Pair with [Floor Scrub](../../03-04-03-well-known-ground-cover-groves/03-04-03-04-floor-scrub/README.md) or exposed terrain.
* Works well in arid and semi-arid groves.
* Sapling-like variants can imply regrowth or young woodland without requiring full tree placement.
