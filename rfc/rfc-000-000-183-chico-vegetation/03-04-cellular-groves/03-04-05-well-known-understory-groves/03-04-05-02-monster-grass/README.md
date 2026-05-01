# 3.4.5.2: Monster Grass

Monster Grass is an extreme understory grass grove derived from [Tall Grass](../../03-04-04-well-known-tufts-groves/03-04-04-02-tall-grass/README.md) and [Braid Grass](../03-04-05-01-braid-grass/README.md). It represents oversized grass masses in the `2m–5m` range, where blades are tall enough to read as environmental structure rather than simple vegetation.

It should feel wet, heavy, and overgrown: thick blade clusters rise above the player, droop under their own mass, and overlap into semi-solid walls of green. Unlike Braid Grass, Monster Grass should not read as delicate or woven. It should feel broad, oppressive, and terrain-shaping.

Good for giant jungle clearings, fantasy wetlands, elder-tree understory, hidden paths, monster habitats, swamp margins, and exaggerated tropical or prehistoric biomes.

```rust
pub enum MonsterGrassCell {
    GiantWetBlade(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.25,
        },
        item: MonsterGrass {
            height: 2.00..6.00,
            width: 0.70..1.60,
            blade_count: 8..=18,
            droop: 0.25..0.70,
            palette_mix: [
                [deep_green..wet_green],
                [blue_green..dark_green],
                [emerald_green..fresh_green],
            ],
        },
    }),
    BroadJungleBlade(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.25,
        },
        item: MonsterGrass {
            height: 2.50..5.00,
            width: 0.90..2.20,
            blade_count: 6..=14,
            droop: 0.35..0.85,
            palette_mix: [
                [lush_green..bright_green],
                [wet_green..lime_green],
                [dark_green..blue_green],
            ],
        },
    }),
    PaleGiantReed(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.25,
        },
        item: MonsterGrass {
            height: 2.00..4.50,
            width: 0.60..1.40,
            blade_count: 6..=16,
            droop: 0.15..0.50,
            palette_mix: [
                [yellow_green..pale_straw],
                [dry_green..tan_green],
                [light_green..fresh_green],
            ],
        },
    }),
    RedRibbedBlade(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.45,
        },
        item: MonsterGrass {
            height: 2.20..4.20,
            width: 0.75..1.80,
            blade_count: 8..=18,
            droop: 0.20..0.65,
            palette_mix: [
                [dark_red..deep_green],
                [copper_red..wet_green],
                [red_green..blue_green],
            ],
        },
    }),
}

impl CellGrove for MonsterGrass {
    type Cell = MonsterGrassCell;

    const CELL_SIZE_RANGE: Range<f32> = 4.0..9.0;
    const DENSITY_RANGE: Range<f32> = 0.18..0.55;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.45;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.02..0.08;
}
```

## Construction

* Use moderate to dense placement, roughly `18%–55%`.
* Use giant blade clusters around `2m–5m`; shorter forms should usually remain [Braid Grass](../03-04-05-01-braid-grass/README.md).
* Use fewer, broader blades than Braid Grass, so each clump reads as heavy vegetation.
* Add strong droop, bend, and asymmetry; blades should sag and lean rather than stand as vertical spikes.
* Allow overlap between neighboring clumps so the grove forms partial walls, screens, and tunnels.
* Prefer wet and saturated palettes: dark green, emerald, blue-green, lime, and occasional red-ribbed variants.
* Use deterministic yaw, scale, blade bend, droop, and width sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where vegetation should alter visibility, path readability, or local navigation.
* Pair with swamp ground cover, [Tropical Tufts](../../03-04-04-well-known-tufts-groves/03-04-04-05-tropical-tufts/README.md), dense canopy, vines, exposed roots, or elder-tree vegetation.
* Works well as cover around hidden ruins, monster dens, humid ravines, and giant-tree bases.
* Keep density lower near important traversal routes unless occlusion is intentional.
* Avoid using as ordinary lush grass; it should feel oversized, dangerous, and habitat-defining.