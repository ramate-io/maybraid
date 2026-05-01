# 3.4.3.1: Huelgoat Pitch

This page is subsection **3.4.3.1** of [RFC-183: Chico Vegetation](../../../README.md)


Huelgoat Pitch is a low, smooth ground-cover grove based on shallow [bump outs](../../../03-03-ground-cover/03-03-01-bump-outs/README.md). It should read as mossy, soft, and continuous, closely following the underlying terrain with only slight vertical lift.

Good for damp forests, riparian shade, temperate groves, old stone regions, and sparse woodland understory.

```rust
pub enum HuelgoatPitchCell {
    BumpOut(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.75,
            steepness: 0.0..0.45,
        },
        item: BumpOut {
            noise: NoiseProfile::LowSmooth,
            height: 0.05..0.10,
            collide: true,
            palette_mix: [
                dark_green..light_green,
            ],
            flecking_mix: [
                Flecking {
                    kind: FleckingKind::Snowfall,
                    strength: Minimal,
                    ..Snowfall::common_flecking(world_size)
                },
            ],
        },
    }),
}

impl CellGrove for HuelgoatPitch {
    type Cell = HuelgoatPitchCell;

    const CELL_SIZE_RANGE: Range<f32> = 50.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.60..0.80;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.25;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.005..0.020;
}
```

**Construction**

* Use a low, smooth bump out with height around `5cm–10cm`.
* Closely follow the underlying terrain normal and terrain SDF.
* Player collision should use the bumped surface, so the player stands on the pitch rather than visually sinking into it.
* Use moderate to high density: roughly `60%–80%` cell activation.
* Use internal cells around `50m–100m`, preferably chosen as even subdivisions of the parent grove cell.
* At low LOD, collapse the internal cell size to the full grove cell.
* Pair with sparse [Tufts](../../../03-03-ground-cover/03-03-02-tufts/README.md) for additional volume detail.
* Flecking should be minimal and generally limited to snowfall.


