# Flecking Bed

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** ground-cover grove (see section 3.4.3 in the main RFC).


Flecking Bed is a soft, non-colliding ground-cover grove based on moderate [bump outs](../../../README.md#331-bump-outs). It should read as a visual vegetation layer rather than physical terrain, allowing the player to sink through it.

Good for wildflower fields, meadow floors, heath, moss beds, flowering understory, and seasonal ground-cover blooms.

```rust
pub enum FleckingBedCell {
    BumpOut(Bucket {
        weight: 1.0,
        item: BumpOut {
            noise: NoiseProfile::Moderate,
            height: 0.10..0.25,
            collide: false,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
            flecking_mix: [
                Flecking {
                    kind: FleckingKind::Bloom,
                    strength: ModerateToHigh,
                    season_weight: High,
                    longitude_weight: Low,
                    altitude_weight: LowToModerate,
                    ..Default::default()
                },
            ],
        },
    }),
}

impl CellGrove for FleckingBed {
    type Cell = FleckingBedCell;

    const CELL_SIZE_RANGE: Range<f32> = 50.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.60..0.85;

    // Normalized fraction of max world height.
    const ELEVATION_RANGE: Range<f32> = 0.0..0.80;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.35;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.25..0.55;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.040;
}
```

**Construction**

* Use a moderate bump out with height around `10cm–25cm`.
* Apply moderate noise, so the surface reads as uneven vegetation rather than smooth terrain.
* Do not enable collision; the player should visually sink through this layer.
* Use moderate to high density, roughly `60%–85%` cell activation.
* Use internal cells around `50m–100m`, preferably even subdivisions of the parent grove cell.
* At low LOD, collapse the internal cell size to the full grove cell.
* Pair well with any tufting pattern, especially sparse flowering tufts or dry brush.

**Flecking**

* Strong seasonal flecking is encouraged.
* Bloom colors may include white, yellow, pink, purple, orange, or pale blue.
* Flecking strength should vary by season and optionally by longitude or altitude.
* Snow flecking may be layered separately, but bloom flecking is the defining feature.
