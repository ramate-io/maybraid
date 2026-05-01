# 3.4.4.1: Common Tufts

Common Tufts are sparse, low-to-moderate grass clumps used as a lightweight volumetric layer over terrain and ground cover. They use the [Tuft](../../../03-01-stalk-and-ball-stick-trees/03-01-02-ball-components/03-01-02-06-tufts/README.md) construction and should vary across a few material and shape varietals rather than using a single repeated appearance.

Good for overlaying detail on existing ground cover where ground-cover likelihood is high. Also, useful as the primary grass layer in arid regions.

```rust
pub enum CommonTuftsCell {
    ShortGreen(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.80,
            steepness: 0.0..0.70,
        },
        item: Tuft {
            height: 0.10..0.25,
            width: 0.08..0.20,
            palette_mix: [dark_green..light_green],
        },
    }),
    DryScrub(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.90,
            steepness: 0.0..0.70,
        },
        item: Tuft {
            height: 0.15..0.40,
            width: 0.08..0.25,
            palette_mix: [vibrant_yellow_green..dry_yellow_green],
        },
    }),
    TallWild(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.60,
            steepness: 0.0..0.70,
        },
        item: Tuft {
            height: 0.30..0.50,
            width: 0.12..0.30,
            palette_mix: [green..pale_green],
        },
    }),
}

impl CellGrove for CommonTufts {
    type Cell = CommonTuftsCell;

    const CELL_SIZE_RANGE: Range<f32> = 1.0..3.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.35;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.05..0.15;
}
```

## Construction

* Use tuft heights around `10cm–50cm`.
* Keep density low, roughly `10%–35%`.
* Select among a few varietals using [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md).
* Place using the same offset and constraint strategy as cellular groves.
* Align tuft direction with the terrain normal.
* Apply deterministic yaw and slight scale variation.

```rust
let p = cell_origin + offset;
let normal = terrain_normal(p);

spawn_tuft(
    position = p,
    direction = normal,
    scale = sampled_height,
    yaw = TAU * noise(seed, ROTATION_SALT),
);
```

## Usage

* Overlay on [Huelgoat Pitch](../../03-04-03-well-known-ground-cover-groves/03-04-03-01-huelgoat-pitch/README.md), [Flecking Bed](../../03-04-03-well-known-ground-cover-groves/03-04-03-02-flecking-bed/README.md), or [Allbed](../../03-04-03-well-known-ground-cover-groves/03-04-03-06-allbed/README.md).
* Use alone in arid or stripped-back regions.
* Reduce density near large tree trunks, rocks, paths, or urbanized surfaces.
