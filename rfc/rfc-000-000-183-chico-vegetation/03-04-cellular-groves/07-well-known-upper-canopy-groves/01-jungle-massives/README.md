# 3.4.7.1: Jungle Massives

Jungle Massives is a moderate-density upper-canopy grove for the true skyline layer above [Jungle Lower Massives](../../06-well-known-lower-canopy-groves/07-jungle-lower-massives/README.md). It uses very large [Jungle Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-jungle-storybook-tree/README.md), [Honu Banyan](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md), [Sope's Banyan](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [Waialea Palm](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md), and rare [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md) variants.

Use this grove when the jungle canopy itself should be enormous and the lower canopy needs to sit beneath it rather than replace it.

```rust
pub enum JungleMassivesCell {
    MassiveJungleStorybook(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.50,
            steepness: 0.0..0.44,
        },
        item: JungleStorybookTree {
            height: 70.0..160.0,
            canopy_density: Dense,
            jungle_growth_density: Dense,
            stick_palette_mix: [[dark_jungle_bark..wet_brown], [moss_bark..dark_bark]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    MassiveHonuBanyan(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.46,
            steepness: 0.0..0.38,
        },
        item: HonuBanyan {
            height: 70.0..200.0,
            canopy_density: Dense,
            descender_frequency: Dense,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    MassiveSopesBanyan(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.44,
            steepness: 0.0..0.42,
        },
        item: SopesBanyan {
            height: 60.0..220.0,
            canopy_density: Dense,
            descender_frequency: Dense,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[dark_green..deep_green], [wet_green..fresh_green]],
        },
    }),
}

impl CellGrove for JungleMassives {
    type Cell = JungleMassivesCell;

    const CELL_SIZE_RANGE: Range<f32> = 28.0..60.0;
    const DENSITY_RANGE: Range<f32> = 0.16..0.34;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.006..0.026;
}
```

## Construction

* Use moderate upper-canopy placement, roughly `16%-34%`.
* Keep Jungle Storybook Tree and Honu Banyan common; use Sope's Banyan less often.
* Keep variants taller than lower-canopy massives, so this grove owns the skyline.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Pair with Jungle Lower Massives, Unending Jungle, Tropical Thicket, vines, and wet ground cover.
* Use where the forest should feel ancient, humid, and vertically deep.
* Avoid dry or temperate contexts unless the biome deliberately wants a displaced jungle giant.
