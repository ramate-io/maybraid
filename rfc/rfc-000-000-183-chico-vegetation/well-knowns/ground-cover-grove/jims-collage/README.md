# Jim's Collage

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** ground-cover grove (see section 3.4.3 in the main RFC).


Jim's Collage is a mixed ground-cover grove that evenly blends [Huelgoat Pitch](../huelgoat-pitch/README.md) and [Flecking Bed](../flecking-bed/README.md). It provides both a grounded mossy layer and a more decorative, seasonal visual layer.

Good for mixed woodland floors, meadow-forest transitions, garden-like groves, riparian clearings, and areas where ground cover should feel varied without introducing many distinct systems.

```rust
pub enum JimsCollageCell {
    HuelgoatPitch(Bucket {
        weight: 1.0,
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 1.0,
        item: FleckingBedCell,
    }),
}

impl CellGrove for JimsCollage {
    type Cell = JimsCollageCell;

    const CELL_SIZE_RANGE: Range<f32> = 50.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.60..0.85;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.80;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.40;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.45;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.035;
}
```

**Construction**

* Use an even split between `HuelgoatPitch` and `FleckingBed`.
* Maintain moderate to high density, roughly `60%–85%`.
* Allow Huelgoat cells to provide low colliding ground softness.
* Allow Flecking Bed cells to provide seasonal bloom and visual softness.
* Use the same internal cell sizing strategy as both parent types: typically `50m–100m`, fit to even subdivisions.
* At low LOD, collapse internal cell size to the full grove cell.
