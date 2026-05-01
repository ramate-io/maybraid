# 3.4.5.1: Braid Grass

Braid Grass is the understory equivalent of [Tall Grass](../../04-well-known-tufts-groves/02-tall-grass/README.md). It represents dense, human-height grass masses in the `1m-3m` range, where individual clumps are tall enough to read as understory vegetation rather than ground or tuft detail.

It should feel woven, layered, and semi-opaque: long blade clusters overlap, cross, and lean through one another, so the grove reads as a braided wall of grass rather than a field of isolated tufts.

Good for riverbanks, humid valleys, jungle margins, overgrown clearings, abandoned paths, wet meadows, and dense transitions between ground cover and larger shrubs.

```rust
pub enum BraidGrassCell {
    DeepGreenBlade(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.75,
            steepness: 0.0..0.60,
        },
        item: BraidGrass {
            height: 1.00..2.20,
            width: 0.35..0.85,
            blade_count: 12..=28,
            braid_twist: 0.10..0.35,
            palette_mix: [
                [deep_green..wet_green],
                [dark_green..emerald_green],
                [blue_green..fresh_green],
            ],
        },
    }),
    PaleReedBlade(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.75,
            steepness: 0.0..0.60,
        },
        item: BraidGrass {
            height: 1.20..2.60,
            width: 0.30..0.70,
            blade_count: 10..=22,
            braid_twist: 0.05..0.25,
            palette_mix: [
                [yellow_green..pale_straw],
                [dry_green..light_green],
                [tan_green..fresh_green],
            ],
        },
    }),
    JungleBlade(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.30,
        },
        item: BraidGrass {
            height: 1.60..3.00,
            width: 0.45..1.00,
            blade_count: 18..=36,
            braid_twist: 0.20..0.50,
            palette_mix: [
                [lush_green..bright_green],
                [wet_green..lime_green],
                [blue_green..deep_green],
            ],
        },
    }),
    RedEdgeBlade(Bucket {
        weight: 0.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.60,
        },
        item: BraidGrass {
            height: 1.00..2.00,
            width: 0.30..0.75,
            blade_count: 10..=24,
            braid_twist: 0.10..0.30,
            palette_mix: [
                [red_green..deep_green],
                [copper_red..yellow_green],
                [dark_red..wet_green],
            ],
        },
    }),
}

impl CellGrove for BraidGrass {
    type Cell = BraidGrassCell;

    const CELL_SIZE_RANGE: Range<f32> = 2.5..6.0;
    const DENSITY_RANGE: Range<f32> = 0.35..0.75;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.03..0.10;
}
```

## Construction

* Use dense placement, roughly `35%–75%`.
* Use blade clusters around `1m–3m`; shorter forms should usually remain [Tall Grass](../../04-well-known-tufts-groves/02-tall-grass/README.md).
* Build each clump from many long, narrow blade planes or frond-like strips.
* Lean neighboring blades in slightly different directions so silhouettes interlace.
* Use crossing and twist to create the braided read, but avoid making the structure look like rope.
* Prefer multiple palette ranges per varietal, especially wet greens, blue-greens, pale reeds, and occasional red-edged tropical blades.
* Use deterministic yaw, scale, bend, and twist sampling.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use where grass should become a navigational or visual understory layer rather than surface detail.
* Pair with sparse [Tropical Tufts](../../04-well-known-tufts-groves/05-tropical-tufts/README.md), reeds, wet soil, jungle trees, or riparian forest edges.
* Works well as screening vegetation along paths, creeks, clearings, and transition zones.
* Use lower density near player paths unless the design wants occlusion or maze-like movement.
* Avoid using as ordinary field grass; it should read as tall, close, and immersive.