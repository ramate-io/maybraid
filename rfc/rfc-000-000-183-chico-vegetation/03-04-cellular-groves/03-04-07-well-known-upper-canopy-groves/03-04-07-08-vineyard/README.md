# 3.4.7.8: Vineyard

Vineyard is a moderate-density cultivated grove with low cell offset. It uses low [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-07-rory-s-head-trained/README.md) variants at `1.5m-3m`, turning the trained crown logic into rows of vine-like shrubs.

```rust
pub enum VineyardCell {
    TrainedVineRory(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.68,
            steepness: 0.0..0.34,
        },
        item: RoryHeadTrained {
            height: 1.5..3.0,
            canopy_density: Sparse,
            canopy_spread: 1.0..2.4,
            stick_palette_mix: [[vine_bark..red_brown], [weathered_bark..gray_brown]],
            canopy_palette_mix: [[grape_green..fresh_green], [deep_green..yellow_green]],
            fruiting_texture_mix: [[grape_purple..blue_black], [pale_grape..green_gold]],
        },
    }),
}

impl CellGrove for Vineyard {
    type Cell = VineyardCell;

    const CELL_SIZE_RANGE: Range<f32> = 3.0..6.0;
    const DENSITY_RANGE: Range<f32> = 0.28..0.50;

    const OFFSET_RANGE: Range<f32> = 0.0..0.14;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.01..0.08;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.028;
}
```

## Construction

* Use moderate-density placement, roughly `28%-50%`.
* Keep offset very low for cultivated row behavior.
* Use grape-like fruiting texture mixes and woody vine stick palettes.
* Prefer flat or gently rolling agricultural terrain.
