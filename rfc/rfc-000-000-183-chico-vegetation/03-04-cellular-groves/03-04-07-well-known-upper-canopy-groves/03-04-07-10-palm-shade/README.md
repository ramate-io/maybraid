# 3.4.7.10: Palm Shade

Palm Shade is a low-density upper-canopy grove using common [Waialea Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-08-waialea-palm/README.md) variants at `8m-40m` and common [Date Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-09-date-palm/README.md) variants at `6m-20m`.

```rust
pub enum PalmShadeCell {
    TallWaialeaPalm(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.46,
            steepness: 0.0..0.56,
        },
        item: WaialeaPalm {
            height: 8.0..40.0,
            crown_density: Moderate,
            stick_palette_mix: [[palm_bark..tan_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[lush_green..bright_green], [wet_green..lime_green]],
        },
    }),
    ShadeDatePalm(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.52,
            steepness: 0.0..0.42,
        },
        item: DatePalm {
            height: 6.0..20.0,
            crown_density: Moderate,
            stick_palette_mix: [[palm_bark..date_trunk], [tan_bark..dry_brown]],
            canopy_palette_mix: [[palm_green..olive_green], [fresh_green..yellow_green]],
        },
    }),
}

impl CellGrove for PalmShade {
    type Cell = PalmShadeCell;

    const CELL_SIZE_RANGE: Range<f32> = 14.0..34.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.32;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.036;
}
```

## Construction

* Use low-density placement, roughly `8%-24%`.
* Keep Waialea Palm and Date Palm evenly common.
* Let Waialea Palm provide the tall shade columns and Date Palm provide lower oasis mass.
* Favor warm, wet, coastal, oasis, or tropical margins.
