# 3.4.4.4: Wild Grass

Wild Grass is a dense, colorful tuft grove using the [Tuft](../../../03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md) construction. It represents tall grass in the `50cm–100cm` range with many varietals and strong color variation.

Good for valleys, prairies, tropical fields, meadow edges, and open terrain.

```rust
pub enum WildGrassCell {
    MeadowGreen(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.60,
            steepness: 0.0..0.65,
        },
        item: Tuft {
            height: 0.50..0.90,
            width: 0.15..0.35,
            palette_mix: [
                [deep_green..light_green],
                [yellow_green..spring_green],
                [olive_green..dark_green],
            ],
        },
    }),
    GoldenGrass(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.70,
            steepness: 0.0..0.55,
        },
        item: Tuft {
            height: 0.60..1.00,
            width: 0.12..0.30,
            palette_mix: [
                [yellow_green..gold],
                [pale_straw..warm_yellow],
                [dry_green..light_brown],
            ],
        },
    }),
    RedPrairie(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.40,
            steepness: 0.0..0.35,
        },
        item: Tuft {
            height: 0.60..1.00,
            width: 0.15..0.35,
            palette_mix: [
                [red_brown..deep_rust],
                [orange_brown..dark_red],
                [dry_green..yellow_green],
            ],
        },
    }),
    BlueTropical(Bucket {
        weight: 0.8,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.40,
            steepness: 0.0..0.35,
        },
        item: Tuft {
            height: 0.60..0.95,
            width: 0.15..0.35,
            palette_mix: [
                [blue_green..bright_green],
                [deep_teal..light_green],
                [dark_green..wet_green],
            ],
        },
    }),
    PaleField(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.60,
            steepness: 0.0..0.35,
        },
        item: Tuft {
            height: 0.50..0.85,
            width: 0.12..0.28,
            palette_mix: [
                [pale_straw..dry_green],
                [cream_yellow..light_brown],
                [silver_green..olive_green],
            ],
        },
    }),
    BloomingGrass(Bucket {
        weight: 0.7,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.70,
            steepness: 0.0..0.35,
        },
        item: Tuft {
            height: 0.50..0.90,
            width: 0.15..0.35,
            palette_mix: [
                [green..flower_flecked],
                [yellow_green..soft_pink],
                [light_green..white_bloom],
                [deep_green..violet_flecked],
            ],
        },
    }),
}

impl CellGrove for WildGrass {
    type Cell = WildGrassCell;

    const CELL_SIZE_RANGE: Range<f32> = 1.0..2.5;
    const DENSITY_RANGE: Range<f32> = 0.65..0.90;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.20..0.45;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.04..0.14;
}
```

## Construction

* Use tuft heights around `50cm–100cm`.
* Keep placement dense, roughly `65%–90%`.
* Select among many varietals using [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md).
* Allow stronger palette variation than [Tall Grass](../02-tall-grass/README.md).
* Align growth mostly upward, with partial terrain-normal influence.
* Apply deterministic yaw, scale, and mild lean variation.

```rust
let p = cell_origin + offset;
let normal = terrain_normal(p);

let direction = normalize(mix(Vec3::Y, normal, 0.25));

spawn_tuft(
    position = p,
    direction,
    scale = sampled_height,
    yaw = TAU * noise(seed, ROTATION_SALT),
);
```

## Usage

* Use in valleys, prairies, fields, and tropical open regions.
* Pair with [Flecking Bed](../../03-well-known-ground-cover-groves/02-flecking-bed/README.md) for flowering meadow effects.
* Pair with [Grassy Mounds](../../03-well-known-ground-cover-groves/05-grassy-mounds/README.md) for rolling grassland texture.
* Avoid dense placement on steep slopes, roads, or exposed urban surfaces.
