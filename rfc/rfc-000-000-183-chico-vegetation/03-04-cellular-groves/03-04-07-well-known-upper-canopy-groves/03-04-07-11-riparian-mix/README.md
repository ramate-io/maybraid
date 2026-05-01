# 3.4.7.11: Riparian Mix

Riparian Mix extends [Riparian General](../03-04-07-04-riparian-general/README.md) by adding common conifer variants to the river-corridor blend. It uses common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md), [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md), [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-14-friend-s-conifer/README.md), and [Temperate Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-15-temperate-conifer/README.md) variants.

```rust
pub enum RiparianMixCell {
    RiparianMixBraidOak(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.44, steepness: 0.0..0.36 },
        item: BraidOak {
            height: 5.0..15.0,
            canopy_density: Moderate,
            stick_palette_mix: [[wet_oak_bark..dark_bark], [moss_bark..gray_brown]],
            canopy_palette_mix: [[river_green..fresh_green], [deep_green..yellow_green]],
        },
    }),
    RiparianMixStorybook(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.46, steepness: 0.0..0.42 },
        item: StorybookTree {
            height: 5.0..15.0,
            canopy_density: Moderate,
            stick_palette_mix: [[broadleaf_bark..wet_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[river_green..light_green], [deep_green..fresh_green]],
        },
    }),
    RiparianFriendConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints { elevation: 0.02..0.58, steepness: 0.0..0.50 },
        item: FriendsConifer {
            height: 8.0..18.0,
            canopy_density: Moderate,
            stick_palette_mix: [[conifer_bark..wet_brown], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [river_green..fresh_green]],
        },
    }),
    RiparianTemperateConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints { elevation: 0.02..0.54, steepness: 0.0..0.46 },
        item: TemperateConifer {
            height: 8.0..18.0,
            canopy_density: Moderate,
            stick_palette_mix: [[temperate_bark..wet_brown], [gray_brown..moss_bark]],
            canopy_palette_mix: [[soft_green..deep_green], [river_green..fresh_green]],
        },
    }),
}

impl CellGrove for RiparianMix {
    type Cell = RiparianMixCell;

    const CELL_SIZE_RANGE: Range<f32> = 10.0..24.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.40;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.046;
}
```

## Construction

* Use moderate-density placement, roughly `18%-40%`.
* Keep broadleaf and conifer variants all common for a mixed riverbank canopy.
* Use wet bark, moss, fresh green, and conifer blue-green palettes.
* Favor low elevation and low steepness, while letting conifers reach slightly cooler banks.
