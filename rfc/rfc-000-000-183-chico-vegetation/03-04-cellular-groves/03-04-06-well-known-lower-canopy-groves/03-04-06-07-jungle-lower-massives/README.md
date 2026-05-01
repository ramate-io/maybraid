# 3.4.6.7: Jungle Lower Massives

Jungle Lower Massives is a moderate-density lower-canopy grove for forests where the upper canopy is formed by very tall, very large trees. It uses common [Jungle Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-jungle-storybook-tree/README.md) and [Honu Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-05-honu-banyan/README.md), less common [Sope's Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-06-sope-s-banyan/README.md) and [Waialea Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-08-waialea-palm/README.md), and rare [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md) forms at `10m-20m`.

It should feel like a massive subcanopy: large jungle trees and palms occupy the band below truly giant upper-canopy trees, giving the forest depth without competing for the very top of the skyline.

Good for elder jungle, rainforest interiors, vast banyan forests, humid valleys, fantasy rainforest cities, and any biome where the upper canopy is so large that normal lower-canopy trees feel too small.

```rust
pub enum JungleLowerMassivesCell {
    LowerMassiveJungleStorybook(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.54,
            steepness: 0.0..0.54,
        },
        item: JungleStorybookTree {
            height: 10.0..20.0,
            canopy_density: Dense,
            jungle_growth_density: Moderate,
            stick_palette_mix: [[dark_jungle_bark..wet_brown], [moss_bark..dark_bark]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    LowerMassiveHonuBanyan(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.50,
            steepness: 0.0..0.46,
        },
        item: HonuBanyan {
            height: 10.0..20.0,
            canopy_density: Dense,
            descender_frequency: Moderate,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    LowerMassiveSopesBanyan(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.48,
            steepness: 0.0..0.50,
        },
        item: SopesBanyan {
            height: 10.0..20.0,
            canopy_density: Dense,
            descender_frequency: Moderate,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[dark_green..deep_green], [wet_green..fresh_green]],
        },
    }),
    LowerMassiveWaialeaPalm(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.44,
            steepness: 0.0..0.62,
        },
        item: WaialeaPalm {
            height: 10.0..20.0,
            crown_density: Dense,
            stick_palette_mix: [[palm_bark..tan_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[lush_green..bright_green], [wet_green..lime_green]],
        },
    }),
    RareLowerMassiveBraidOak(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.02..0.50,
            steepness: 0.0..0.52,
        },
        item: BraidOak {
            height: 10.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[wet_oak_bark..dark_bark], [moss_bark..green_brown]],
            canopy_palette_mix: [[deep_green..fresh_green], [wet_green..yellow_green]],
        },
    }),
}

impl CellGrove for JungleLowerMassives {
    type Cell = JungleLowerMassivesCell;

    const CELL_SIZE_RANGE: Range<f32> = 16.0..30.0;
    const DENSITY_RANGE: Range<f32> = 0.20..0.42;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.36;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.040;
}
```

## Construction

* Use moderate-density placement, roughly `20%-42%`.
* Keep Jungle Storybook Tree and Honu Banyan common at `10m-20m`; they provide the main lower-canopy mass.
* Use Sope's Banyan and Waialea Palm less often for distinctive banyan and palm silhouettes.
* Use Braid Oak rarely, as an unusual broad, braided accent inside the jungle layer.
* Keep palettes wet, saturated, and shaded, so the grove reads below a very tall upper canopy.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use below elder jungle trees, huge rainforest trees, and other very tall upper-canopy constructions.
* Pair with Tropical Thicket, Tropical Undergrowth, vines, wet ground cover, and large root systems.
* Works when ordinary `3m-8m` lower-canopy groves are too short to fill the visual middle of the forest.
* Avoid using it as the tallest layer; these trees are massive for lower canopy, but still subordinate to the dominant canopy above.
