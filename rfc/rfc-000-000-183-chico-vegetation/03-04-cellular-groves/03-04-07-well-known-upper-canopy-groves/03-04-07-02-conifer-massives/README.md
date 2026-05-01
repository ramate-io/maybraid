# 3.4.7.2: Conifer Massives

Conifer Massives is a low-density upper-canopy grove for giant evergreen forests above [Conifer Lower Massives](../../03-04-06-well-known-lower-canopy-groves/03-04-06-08-conifer-lower-massives/README.md). It uses very large [Liam's Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-02-liam-s-conifer/README.md), [Northern Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-11-northern-conifer/README.md), [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-14-friend-s-conifer/README.md), and [Temperate Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-15-temperate-conifer/README.md) variants.

Use this grove where upper-canopy conifers should tower over substantial lower-canopy evergreen growth.

```rust
pub enum ConiferMassivesCell {
    MassiveNorthernConifer(Bucket {
        weight: 1.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.28..0.96,
            steepness: 0.0..0.70,
        },
        item: NorthernConifer {
            height: 70.0..200.0,
            canopy_density: Dense,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..conifer_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [deep_green..dark_green]],
        },
    }),
    MassiveFriendsConifer(Bucket {
        weight: 1.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.22..0.90,
            steepness: 0.0..0.64,
        },
        item: FriendsConifer {
            height: 100.0..130.0,
            canopy_density: Dense,
            stick_palette_mix: [[conifer_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [dark_green..fresh_green]],
        },
    }),
    MassiveLiamsConifer(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.30..0.98,
            steepness: 0.0..0.76,
        },
        item: LiamsConifer {
            height: 25.0..130.0,
            canopy_density: Moderate,
            stick_palette_mix: [[conifer_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [dark_green..fresh_green]],
        },
    }),
    MassiveTemperateConifer(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.16..0.76,
            steepness: 0.0..0.58,
        },
        item: TemperateConifer {
            height: 40.0..120.0,
            canopy_density: Moderate,
            stick_palette_mix: [[temperate_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[soft_green..deep_green], [blue_green..fresh_green]],
        },
    }),
}

impl CellGrove for ConiferMassives {
    type Cell = ConiferMassivesCell;

    const CELL_SIZE_RANGE: Range<f32> = 30.0..70.0;
    const DENSITY_RANGE: Range<f32> = 0.06..0.20;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.28;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.005..0.024;
}
```

## Construction

* Use low-density placement, roughly `6%-20%`.
* Keep Northern Conifer and Friend's Conifer most common; mix Liam's Conifer and Temperate Conifer for silhouette variation.
* Bias placement toward cool, upland, or mountain conditions.
* Keep cell sizes large, so each conifer has enough visual room to read as an upper-canopy tree.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Pair with Conifer Lower Massives, Conifer Sapling, Huelgoat Pitch, Allbed, moss, and rock.
* Use for giant evergreen stands and mountain forests.
* Avoid high density; these should feel like major skyline trees, not a packed hedge.
