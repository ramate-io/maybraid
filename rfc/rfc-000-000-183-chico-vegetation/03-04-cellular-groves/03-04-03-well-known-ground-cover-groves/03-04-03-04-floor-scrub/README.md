# 3.4.3.4: Floor Scrub

This page is subsection **3.4.3.4** of [RFC-183: Chico Vegetation](../../../README.md)


Floor Scrub is a low-density variant of [Jim's Collage](../03-04-03-03-jim-s-collage/README.md#3433-jims-collage). It uses the same basic split between [Huelgoat Pitch](../03-04-03-01-huelgoat-pitch/README.md#3431-huelgoat-pitch) and [Flecking Bed](../03-04-03-02-flecking-bed/README.md#3432-flecking-bed), but reduces coverage and uses smaller internal cells for a patchier, more exposed ground layer.

Good for arid regions, sparse woodland, stripped-back understory, chaparral edges, dry groves, and disturbed terrain.

```rust
pub enum FloorScrubCell {
    HuelgoatPitch(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.45,
        },
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.45,
        },
        item: FleckingBedCell,
    }),
}

impl CellGrove for FloorScrub {
    type Cell = FloorScrubCell;

    const CELL_SIZE_RANGE: Range<f32> = 15.0..20.0;
    const DENSITY_RANGE: Range<f32> = 0.20..0.45;

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

