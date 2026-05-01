# 3.4.7.18: Christmas Taiga

Christmas Taiga is a moderate-density upper-canopy grove using common [Northern Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-11-northern-conifer/README.md) variants at `8m-20m`.

```rust
pub enum ChristmasTaigaCell {
    ChristmasNorthernConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.34..0.96,
            steepness: 0.0..0.76,
        },
        item: NorthernConifer {
            height: 8.0..20.0,
            canopy_density: Dense,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..conifer_bark]],
            canopy_palette_mix: [[christmas_green..deep_green], [blue_green..dark_green]],
        },
    }),
    HighBandNorthernConifer(Bucket {
        weight: 0.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.42..1.0,
            steepness: 0.0..0.82,
        },
        item: NorthernConifer {
            height: 8.0..20.0,
            canopy_density: Dense,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..conifer_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [deep_green..dark_green]],
        },
    }),
}

impl CellGrove for ChristmasTaiga {
    type Cell = ChristmasTaigaCell;

    const CELL_SIZE_RANGE: Range<f32> = 10.0..22.0;
    const DENSITY_RANGE: Range<f32> = 0.20..0.42;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.28;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.044;
}
```

## Construction

* Use moderate-density placement, roughly `20%-42%`.
* Keep Northern Conifer common, with a colder high-band variant for upper elevations.
* Bias toward high elevation and cold terrain.
* Keep canopy mixes on base tree colors; snow or frost should come from separate flecking concerns.
* Pair with alpine ground cover, moss, rocks, and sparse understory.
