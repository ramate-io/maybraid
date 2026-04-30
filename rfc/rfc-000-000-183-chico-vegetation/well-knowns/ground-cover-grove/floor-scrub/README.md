# Floor Scrub

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** ground-cover grove (see section 3.4.3 in the main RFC).


Floor Scrub is a low-density variant of [Jim's Collage](../jims-collage/README.md). It uses the same basic split between [Huelgoat Pitch](../huelgoat-pitch/README.md) and [Flecking Bed](../flecking-bed/README.md), but reduces coverage and uses smaller internal cells for a patchier, more exposed ground layer.

Good for arid regions, sparse woodland, stripped-back understory, chaparral edges, dry groves, and disturbed terrain.

```rust
pub enum FloorScrubCell {
    HuelgoatPitch(Bucket {
        weight: 1.0,
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 1.0,
        item: FleckingBedCell,
    }),
}

impl CellGrove for FloorScrub {
    type Cell = FloorScrubCell;

    const CELL_SIZE_RANGE: Range<f32> = 15.0..20.0;
    const DENSITY_RANGE: Range<f32> = 0.20..0.45;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.85;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.45;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.015..0.050;
}
```

**Construction**

* Use a low-density split between `HuelgoatPitch` and `FleckingBed`.
* Keep density around `20%–45%`.
* Use internal cells around `15m`, fit to even subdivisions of the parent grove cell.
* At low LOD, collapse internal cell size to the full grove cell.
* Prefer weaker bump-out heights and lighter flecking than Jim's Collage.
* Pair well with sparse tufts, dry brush, or exposed terrain detail.
