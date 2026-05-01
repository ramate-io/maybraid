# 3.4.6.9: Temperate Lower Massives

Temperate Lower Massives is a low-density lower-canopy grove for forests where very tall and large trees dominate the upper canopy. It uses common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md) and [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md) forms, plus rare [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md) variants, at `10m-20m`.

It should provide a broad temperate subcanopy beneath enormous upper-canopy trees. The grove stays low density so each large lower-canopy tree has enough room to read as a mature form rather than a crowded stand.

Good for old temperate forests, giant oak woods, park-like elder groves, village-edge forests, fantasy broadleaf interiors, and any scene where a very tall canopy needs a substantial middle tree layer.

```rust
pub enum TemperateLowerMassivesCell {
    LowerMassiveBraidOak(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.68,
            steepness: 0.0..0.50,
        },
        item: BraidOak {
            height: 10.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[oak_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..fresh_green], [dark_green..light_green]],
        },
    }),
    LowerMassiveStorybook(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.72,
            steepness: 0.0..0.56,
        },
        item: StorybookTree {
            height: 10.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[broadleaf_bark..brown_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[broadleaf_green..light_green], [deep_green..yellow_green]],
        },
    }),
    RareLowerMassiveRory(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.06..0.64,
            steepness: 0.0..0.68,
        },
        item: RoryHeadTrained {
            height: 10.0..20.0,
            canopy_density: Sparse,
            canopy_spread: 2.5..6.0,
            stick_palette_mix: [[weathered_bark..dark_bark], [red_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..fresh_green], [yellow_green..light_green]],
        },
    }),
}

impl CellGrove for TemperateLowerMassives {
    type Cell = TemperateLowerMassivesCell;

    const CELL_SIZE_RANGE: Range<f32> = 18.0..34.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.26;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.036;
}
```

## Construction

* Use low-density placement, roughly `10%-26%`.
* Keep Braid Oak and Storybook Tree common at `10m-20m`; they form the main temperate lower-canopy mass.
* Use Rory's Head-trained rarely for distinctive trained crowns and open silhouettes.
* Use broadleaf bark and green canopy palettes, with enough variation to keep repeated large forms from looking cloned.
* Allow moderate slope, but keep the strongest placement in stable temperate woodland bands.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use beneath very tall broadleaf trees, elder oaks, upper-canopy storybook forms, or enormous mixed-forest trees.
* Pair with Goettingen Follow, High Bush, Low Bush, Riverine Green, Braid Oak groves, grass, moss, and fallen logs.
* Works when ordinary temperate lower-canopy groves are too short to fill the visual middle of a giant forest.
* Avoid high density; these trees should feel mature and spacious beneath the taller canopy above.
