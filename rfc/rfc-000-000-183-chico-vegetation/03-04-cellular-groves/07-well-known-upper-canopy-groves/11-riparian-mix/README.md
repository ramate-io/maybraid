# 3.4.7.11: Riparian Mix

Riparian Mix extends [Riparian General](../04-riparian-general/README.md) by adding common conifer variants to the river-corridor blend. It uses common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md), [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md), [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md), and [Temperate Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/15-temperate-conifer/README.md) variants.

```rust
pub enum RiparianMixCell {
    BankBraidOak(Bucket {
        weight: 0.9,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.38, steepness: 0.0..0.30 },
        item: BraidOak {
            height: 5.0..12.0,
            canopy_density: Dense,
            stick_palette_mix: [[wet_oak_bark..dark_bark], [moss_bark..gray_brown]],
            canopy_palette_mix: [[river_green..fresh_green], [deep_green..yellow_green]],
        },
    }),
    OverbankBraidOak(Bucket {
        weight: 0.6,
        placement_constraints: PlacementConstraints { elevation: 0.02..0.48, steepness: 0.0..0.42 },
        item: BraidOak {
            height: 10.0..18.0,
            canopy_density: Moderate,
            stick_palette_mix: [[oak_bark..wet_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[deep_green..fresh_green], [river_green..yellow_green]],
        },
    }),
    RoundRiparianStorybook(Bucket {
        weight: 0.9,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.46, steepness: 0.0..0.42 },
        item: StorybookTree {
            height: 5.0..15.0,
            canopy_density: Moderate,
            stick_palette_mix: [[broadleaf_bark..wet_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[river_green..light_green], [deep_green..fresh_green]],
        },
    }),
    TallRiparianStorybook(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints { elevation: 0.02..0.52, steepness: 0.0..0.48 },
        item: StorybookTree {
            height: 12.0..22.0,
            canopy_density: Sparse,
            stick_palette_mix: [[broadleaf_bark..wet_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[river_green..light_green], [deep_green..fresh_green]],
        },
    }),
    BankFriendConifer(Bucket {
        weight: 0.8,
        placement_constraints: PlacementConstraints { elevation: 0.02..0.58, steepness: 0.0..0.50 },
        item: FriendsConifer {
            height: 8.0..16.0,
            canopy_density: Dense,
            stick_palette_mix: [[conifer_bark..wet_brown], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [river_green..fresh_green]],
        },
    }),
    ShelteredTemperateConifer(Bucket {
        weight: 0.8,
        placement_constraints: PlacementConstraints { elevation: 0.04..0.62, steepness: 0.0..0.54 },
        item: TemperateConifer {
            height: 10.0..20.0,
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
* Split Braid Oak and Storybook variants into lower bank forms and taller overbank forms.
* Use denser Friend's Conifer near cool banks and looser Temperate Conifer on sheltered margins.
* Use wet bark, moss, fresh green, and conifer blue-green palettes.
* Favor low elevation and low steepness, while letting conifers reach slightly cooler banks.
