# 3.4.6.8: Conifer Lower Massives

Conifer Lower Massives is a low-density lower-canopy grove for forests where the upper canopy is formed by very tall, very large trees. It uses common large conifer variants, including [Liam's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md), [Northern Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/11-northern-conifer/README.md), and [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md) forms at `10m-20m`.

It should provide a sparse but legible evergreen subcanopy beneath truly massive trees. The grove is intentionally low density: these are substantial trees, so each placement should matter.

Good for old conifer forests, mountain giants, boreal elder stands, alpine fantasy forests, deep evergreen valleys, and any scene where the dominant upper canopy is much taller than ordinary conifers.

```rust
pub enum ConiferLowerMassivesCell {
    LowerMassiveLiamsConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.22..0.86,
            steepness: 0.0..0.68,
        },
        item: LiamsConifer {
            height: 10.0..20.0,
            canopy_density: Moderate,
            stick_palette_mix: [[conifer_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [dark_green..fresh_green]],
        },
    }),
    LowerMassiveNorthernConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.26..0.92,
            steepness: 0.0..0.76,
        },
        item: NorthernConifer {
            height: 10.0..20.0,
            canopy_density: Dense,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..conifer_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [deep_green..dark_green]],
        },
    }),
    LowerMassiveFriendsConifer(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.20..0.88,
            steepness: 0.0..0.70,
        },
        item: FriendsConifer {
            height: 10.0..20.0,
            canopy_density: Dense,
            stick_palette_mix: [[conifer_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [dark_green..fresh_green]],
        },
    }),
}

impl CellGrove for ConiferLowerMassives {
    type Cell = ConiferLowerMassivesCell;

    const CELL_SIZE_RANGE: Range<f32> = 18.0..34.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.32;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.034;
}
```

## Construction

* Use low-density placement, roughly `8%-24%`.
* Use Liam's Conifer, Northern Conifer, and Friend's Conifer evenly at `10m-20m`.
* Keep all conifer variants common within the sparse distribution; low density should provide the open spacing.
* Use cool evergreen palettes and substantial canopy density, so each tree reads as a meaningful lower-canopy mass.
* Bias placement toward upland, cool, or mountain terrain while allowing moderate-to-steep slopes.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use beneath very tall conifers, elder evergreens, mountain giants, or other oversized upper-canopy trees.
* Pair with Conifer Sapling, Arid Conifer Sapling, Huelgoat Pitch, Allbed, moss, rocks, and fallen logs.
* Works when the scene needs a lower canopy of substantial conifers without closing the forest floor.
* Avoid using it in lush jungle, lowland broadleaf forest, or dry scrub unless the biome calls for an evergreen intrusion.
