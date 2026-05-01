# 3.4.7.12: Alpine

Alpine is a moderate-density upper-canopy grove for cold uplands. It uses common [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md) variants and less common [Liam's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md) variants at `10m-40m`.

```rust
pub enum AlpineCell {
    TallAlpineFriendsConifer(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.42..1.0,
            steepness: 0.0..0.68,
        },
        item: FriendsConifer {
            height: 18.0..40.0,
            canopy_density: Dense,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [deep_green..dark_green]],
        },
    }),
    WindlineFriendsConifer(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.62..1.0,
            steepness: 0.0..0.86,
        },
        item: FriendsConifer {
            height: 10.0..22.0,
            canopy_density: Sparse,
            stick_palette_mix: [[wind_barked..cold_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [dark_green..deep_green]],
        },
    }),
    AlpineLiamsConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.50..1.0,
            steepness: 0.0..0.86,
        },
        item: LiamsConifer {
            height: 10.0..40.0,
            canopy_density: Moderate,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..conifer_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [deep_green..dark_green]],
        },
    }),
    NeedleSpireLiamsConifer(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.58..1.0,
            steepness: 0.0..0.92,
        },
        item: LiamsConifer {
            height: 16.0..32.0,
            canopy_density: Sparse,
            stick_palette_mix: [[cold_bark..dark_bark], [stone_gray..conifer_bark]],
            canopy_palette_mix: [[blue_green..dark_green], [cold_green..deep_green]],
        },
    }),
}

impl CellGrove for Alpine {
    type Cell = AlpineCell;

    const CELL_SIZE_RANGE: Range<f32> = 16.0..38.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.38;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.34;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.036;
}
```

## Construction

* Use moderate-density placement, roughly `18%-38%`.
* Keep tall Friend's Conifer common and Liam's Conifer less common.
* Add shorter windline Friend's Conifer and narrow Liam's Conifer spires for exposed ridges.
* Bias strongly toward high elevation and tolerate steep slopes.
* Use cold bark and blue-green needle palettes.
