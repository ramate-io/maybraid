# 3.4.7.9: Date Grove

Date Grove is a moderate-density cultivated upper-canopy grove similar to [Orchard](../03-04-07-07-orchard/README.md), but using [Date Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-09-date-palm/README.md) variants with date fruiting textures.

```rust
pub enum DateGroveCell {
    FruitingDatePalm(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.46,
            steepness: 0.0..0.30,
        },
        item: DatePalm {
            height: 5.0..8.0,
            crown_density: Moderate,
            stick_palette_mix: [[palm_bark..tan_bark], [date_trunk..dry_brown]],
            canopy_palette_mix: [[palm_green..olive_green], [fresh_green..yellow_green]],
            fruiting_texture_mix: [[date_gold..date_brown], [amber_fruit..dark_date]],
        },
    }),
}

impl CellGrove for DateGrove {
    type Cell = DateGroveCell;

    const CELL_SIZE_RANGE: Range<f32> = 8.0..16.0;
    const DENSITY_RANGE: Range<f32> = 0.22..0.42;

    const OFFSET_RANGE: Range<f32> = 0.0..0.18;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.02..0.12;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.032;
}
```

## Construction

* Use moderate-density placement, roughly `22%-42%`.
* Keep offset low, so the grove can read as cultivated.
* Include date-colored fruiting texture mixes.
* Favor warm, flat, irrigated, oasis, or agricultural terrain.
