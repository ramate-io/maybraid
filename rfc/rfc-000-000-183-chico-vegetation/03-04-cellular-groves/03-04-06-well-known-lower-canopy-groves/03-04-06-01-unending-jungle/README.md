# 3.4.6.1: Unending Jungle

Unending Jungle is a moderate-density lower-canopy grove for forests where the primary canopy is much taller, but the middle height band still needs young trees, palms, and banyan-like structure. It uses common mini [Honu Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-05-honu-banyan/README.md), less common mini [Sope's Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-06-sope-s-banyan/README.md), common [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md), small [Jungle Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-jungle-storybook-tree/README.md), rare [Penmarch Torch](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-04-penmarch-torch/README.md), rare [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-07-rory-s-head-trained/README.md), and rare [Waialea Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-08-waialea-palm) forms.

It should feel endless and layered: short jungle trees fill gaps beneath elder or upper-canopy trees without replacing them. Banyan forms provide irregular horizontal mass, Storybook variants provide rounded green fill, and rare palms or torch shapes add vertical punctuation.

Good for deep jungle, rainforest lower canopy, banyan forests, humid ravines, and dense tropical areas where tall trees need a rich subcanopy.

```rust
pub enum UnendingJungleCell {
    SmallHonuBanyan(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.42,
        },
        item: HonuBanyan {
            height: 4.0..6.0,
            canopy_density: Moderate,
            descender_frequency: Sparse,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    SmallSopeBanyan(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.52,
            steepness: 0.0..0.48,
        },
        item: SopesBanyan {
            height: 4.0..6.0,
            canopy_density: Moderate,
            descender_frequency: Sparse,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[dark_green..deep_green], [wet_green..fresh_green]],
        },
    }),
    LowerStorybook(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.58,
            steepness: 0.0..0.64,
        },
        item: StorybookTree {
            height: 3.0..5.0,
            canopy_density: Moderate,
            stick_palette_mix: [[tropical_bark..dark_bark], [green_brown..wet_brown]],
            canopy_palette_mix: [[lush_green..bright_green], [deep_green..fresh_green]],
        },
    }),
    SmallJungleStorybook(Bucket {
        weight: 1.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.58,
        },
        item: JungleStorybookTree {
            height: 6.0..8.0,
            canopy_density: Dense,
            jungle_growth_density: Moderate,
            stick_palette_mix: [[dark_jungle_bark..wet_brown], [moss_bark..dark_bark]],
            canopy_palette_mix: [[deep_green..wet_green], [blue_green..emerald_green]],
        },
    }),
    PenmarchAccent(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.50,
            steepness: 0.0..0.64,
        },
        item: PenmarchTorch {
            height: 3.0..5.0,
            canopy_density: Sparse,
            stick_palette_mix: [[tropical_bark..dark_bark], [green_brown..wet_brown]],
            canopy_palette_mix: [[wet_green..lime_green], [dark_green..fresh_green]],
        },
    }),
    RedJungleTorch(Bucket {
        weight: 0.20,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.48,
            steepness: 0.0..0.58,
        },
        item: PenmarchTorch {
            height: 3.0..5.5,
            canopy_density: Sparse,
            stick_palette_mix: [[red_jungle_bark..copper_red], [wet_burgundy..dark_bark]],
            canopy_palette_mix: [[wet_green..lime_green], [blue_green..fresh_green]],
        },
    }),
    RoryAccent(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.76,
        },
        item: RoryHeadTrained {
            height: 3.0..7.0,
            canopy_density: Sparse,
            canopy_spread: 1.0..2.8,
            stick_palette_mix: [[tropical_bark..dark_bark], [vine_bark..wet_brown]],
            canopy_palette_mix: [[blue_green..deep_green], [yellow_green..fresh_green]],
        },
    }),
    WaialeaPalmAccent(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.70,
        },
        item: WaialeaPalm {
            height: 6.0..9.0,
            crown_density: Moderate,
            stick_palette_mix: [[palm_bark..tan_bark], [wet_brown..green_brown]],
            canopy_palette_mix: [[lush_green..bright_green], [wet_green..lime_green]],
        },
    }),
}

impl CellGrove for UnendingJungle {
    type Cell = UnendingJungleCell;

    const CELL_SIZE_RANGE: Range<f32> = 7.0..14.0;
    const DENSITY_RANGE: Range<f32> = 0.24..0.52;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.015..0.060;
}
```

## Construction

* Use moderate placement, roughly `24%–52%`.
* Keep Honu Banyan and Storybook variants common; they provide the primary lower-canopy mass.
* Add small Jungle Storybook variants at `6m–8m` where the lower canopy should feel wetter, denser, and more entangled.
* Use Sope's Banyan less often for taller, more mystical vertical banyan structure.
* Use Penmarch Torch, Rory's Head-trained, and Waialea Palm rarely as accents.
* Add very rare red-stick torch accents for flashes of color inside the green lower canopy.
* Keep all variants below the main canopy layer; this grove should fill the middle band beneath taller trees.
* Use wet, saturated jungle palettes with occasional yellow-green new growth.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where tall jungle or elder trees need a dense lower canopy underneath.
* Pair with Tropical Thicket, Tropical Undergrowth, vines, wet ground cover, and large upper-canopy trees.
* Works well in ravines, rainforest interiors, and old banyan regions.
* Keep rare accents rare enough that the grove reads as continuous jungle, not a specimen collection.
