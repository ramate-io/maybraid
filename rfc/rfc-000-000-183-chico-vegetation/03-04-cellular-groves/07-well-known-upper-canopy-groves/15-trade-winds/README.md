# 3.4.7.15: Trade Winds

Trade Winds is a low-density tropical upper-canopy grove. It uses less common [Sope's Banyan](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md) and [Honu Banyan](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md) variants at `10m-25m`, common [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md) variants at `10m-20m`, rare taller Storybook variants at `20m-30m`, and rare [Waialea Palm](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md) variants at `10m-40m`.

```rust
pub enum TradeWindsCell {
    TradeStorybook(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.56, steepness: 0.0..0.48 },
        item: StorybookTree {
            height: 10.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[tropical_bark..wet_brown], [gray_brown..dark_bark]],
            canopy_palette_mix: [[fresh_green..bright_green], [deep_green..yellow_green]],
        },
    }),
    TradeSopesBanyan(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.48, steepness: 0.0..0.44 },
        item: SopesBanyan {
            height: 10.0..25.0,
            canopy_density: Moderate,
            descender_frequency: Sparse,
            stick_palette_mix: [[banyan_bark..wet_brown], [green_brown..dark_bark]],
            canopy_palette_mix: [[dark_green..deep_green], [wet_green..fresh_green]],
        },
    }),
    TradeHonuBanyan(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.50, steepness: 0.0..0.42 },
        item: HonuBanyan {
            height: 10.0..25.0,
            canopy_density: Moderate,
            descender_frequency: Sparse,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    RareTallTradeStorybook(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.52, steepness: 0.0..0.50 },
        item: StorybookTree {
            height: 20.0..30.0,
            canopy_density: Dense,
            stick_palette_mix: [[tropical_bark..dark_bark], [wet_brown..gray_brown]],
            canopy_palette_mix: [[lush_green..bright_green], [deep_green..fresh_green]],
        },
    }),
    RareTradeWaialeaPalm(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints { elevation: 0.0..0.44, steepness: 0.0..0.58 },
        item: WaialeaPalm {
            height: 10.0..40.0,
            crown_density: Moderate,
            stick_palette_mix: [[palm_bark..tan_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[lush_green..bright_green], [wet_green..lime_green]],
        },
    }),
}

impl CellGrove for TradeWinds {
    type Cell = TradeWindsCell;

    const CELL_SIZE_RANGE: Range<f32> = 16.0..36.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.34;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.034;
}
```

## Construction

* Use low-density placement, roughly `8%-24%`.
* Keep Storybook Tree common; use banyans less commonly and palms rarely.
* Use taller Storybook variants as rare skyline accents.
* Favor warm, coastal, humid, or wind-shaped tropical terrain.
