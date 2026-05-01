# 3.4.7.7: Orchard

Orchard is a moderate-density upper-canopy grove with low cell offset. It uses compact [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) variants at `5m-10m` and includes fruiting texture variants.

```rust
pub enum OrchardCell {
    FruitingStorybook(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.02..0.62,
            steepness: 0.0..0.30,
        },
        item: StorybookTree {
            height: 5.0..10.0,
            canopy_density: Moderate,
            stick_palette_mix: [[orchard_bark..brown_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[fresh_green..light_green], [deep_green..yellow_green]],
            fruiting_texture_mix: [[apple_red..gold_fruit], [plum_purple..pear_green]],
        },
    }),
    PaleBloomStorybook(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.02..0.60,
            steepness: 0.0..0.28,
        },
        item: StorybookTree {
            height: 5.0..9.0,
            canopy_density: Moderate,
            stick_palette_mix: [[orchard_bark..gray_brown], [tan_bark..brown_bark]],
            canopy_palette_mix: [[pale_blossom..fresh_green], [light_green..yellow_green]],
            fruiting_texture_mix: [[cream_blossom..pink_blossom], [gold_fruit..apple_red]],
        },
    }),
}

impl CellGrove for Orchard {
    type Cell = OrchardCell;

    const CELL_SIZE_RANGE: Range<f32> = 8.0..14.0;
    const DENSITY_RANGE: Range<f32> = 0.24..0.44;

    const OFFSET_RANGE: Range<f32> = 0.0..0.18;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.02..0.12;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.030;
}
```

## Construction

* Use moderate-density placement, roughly `24%-44%`.
* Keep cell offset low, so trees align like tended rows without becoming perfectly rigid.
* Include fruiting texture mixes on all variants.
* Favor flat, cultivated, low-slope terrain.
