Here’s the updated version with a Hawaiian-style reddish varietal added:

---

# 3.4.4.2: Tall Grass

Tall Grass is a dense tuft grove made from the [Tuft](../../../03-01-stalk-and-ball-stick-trees/03-01-02-ball-components/03-01-02-06-tufts/README.md) construction. It represents grass clumps in the `50cm–100cm` range, reserving taller forms for understory systems.

Good for tropical grasslands, riverine edges, wet meadows, and lush transitional regions.

```rust
pub enum TallGrassCell {
    RiverGreen(Bucket {
        weight: 2.0,
        item: Tuft {
            height: 0.50..0.90,
            width: 0.15..0.35,
            palette_mix: [deep_green..light_green],
        },
    }),
    PaleReed(Bucket {
        weight: 1.0,
        item: Tuft {
            height: 0.60..1.00,
            width: 0.12..0.30,
            palette_mix: [yellow_green..pale_straw],
        },
    }),
    TropicalBlade(Bucket {
        weight: 1.0,
        item: Tuft {
            height: 0.70..1.00,
            width: 0.18..0.40,
            palette_mix: [blue_green..bright_green],
        },
    }),
    HawaiianRed(Bucket {
        weight: 1.0,
        item: Tuft {
            height: 0.70..1.00,
            width: 0.18..0.40,
            palette_mix: [red_brown..deep_rust],
        },
    }),
}
```

```rust
impl CellGrove for TallGrass {
    type Cell = TallGrassCell;

    const CELL_SIZE_RANGE: Range<f32> = 1.0..2.5;
    const DENSITY_RANGE: Range<f32> = 0.55..0.85;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.75;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.30;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.04..0.12;
}
```

## Construction

* Use tuft heights around `50cm–100cm`.
* Keep placement dense, roughly `55%–85%`.
* Include multiple varietals, including reddish tropical grasses.
* Align tufts with terrain normal, but keep most growth visually upward.
* Use deterministic yaw and mild height variation.

```rust
let p = cell_origin + offset;
let normal = terrain_normal(p);

let direction = normalize(mix(Vec3::Y, normal, 0.35));

spawn_tuft(
    position = p,
    direction,
    scale = sampled_height,
    yaw = TAU * noise(seed, ROTATION_SALT),
);
```

## Usage

* Use in tropical, wet, or implied riverine regions.
* The reddish varietal works especially well in volcanic, coastal, or dry tropical environments.
* Pair well with [Flecking Bed](../../03-04-03-well-known-ground-cover-groves/03-04-03-02-flecking-bed/README.md) and [Allbed](../../03-04-03-well-known-ground-cover-groves/03-04-03-06-allbed/README.md).
* Avoid dense use on steep slopes, roads, and exposed urbanized terrain.
* Reserve grasses above `100cm` for understory or specialized grove layers.