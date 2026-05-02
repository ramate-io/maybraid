# 3.4.7.3: Temperate Massives

Temperate Massives is a low-density upper-canopy grove for giant broadleaf forests above [Temperate Lower Massives](../../06-well-known-lower-canopy-groves/09-temperate-lower-massives/README.md). It uses very large [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md), [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md), and rare [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md) variants.

Use this grove where the upper canopy should be made from enormous temperate trees, with lower-canopy massives filling the band beneath.

```rust
pub enum TemperateMassivesCell {
    MassiveBraidOak(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.72,
            steepness: 0.0..0.44,
        },
        item: BraidOak {
            height: 28.0..80.0,
            canopy_density: Dense,
            stick_palette_mix: [[oak_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..fresh_green], [dark_green..light_green]],
        },
    }),
    MassiveStorybook(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.04..0.76,
            steepness: 0.0..0.50,
        },
        item: StorybookTree {
            height: 35.0..170.0,
            canopy_density: Dense,
            stick_palette_mix: [[broadleaf_bark..brown_bark], [gray_brown..dark_bark]],
            canopy_palette_mix: [[broadleaf_green..light_green], [deep_green..yellow_green]],
        },
    }),
    RareMassiveRory(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.06..0.66,
            steepness: 0.0..0.60,
        },
        item: RoryHeadTrained {
            height: 50.0..200.0,
            canopy_density: Moderate,
            canopy_spread: 6.0..14.0,
            stick_palette_mix: [[weathered_bark..dark_bark], [red_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..fresh_green], [yellow_green..light_green]],
        },
    }),
}

impl CellGrove for TemperateMassives {
    type Cell = TemperateMassivesCell;

    const CELL_SIZE_RANGE: Range<f32> = 30.0..68.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.22;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.005..0.026;
}
```

## Construction

* Use low-density placement, roughly `8%-22%`.
* Keep Braid Oak and Storybook Tree common; use Rory's Head-trained rarely for distinctive open crowns.
* Use broadleaf palettes with large, shaded canopy masses.
* Keep these variants taller and broader than Temperate Lower Massives.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Pair with Temperate Lower Massives, Goettingen Follow, High Bush, Riverine Green, and temperate ground cover.
* Use in giant oak woods, elder broadleaf forests, and fantasy parklands.
* Avoid crowding; these are skyline trees and need space.
