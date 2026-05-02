# 3.4.6.6: Arid Conifer Sapling

Arid Conifer Sapling is a low-density lower-canopy grove using common dry young [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md) and [Northern Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/11-northern-conifer/README.md) variants at `2m-4m`.

It represents scattered conifer regeneration in dry, exposed terrain: small evergreens break up scrub or rocky ground without forming a closed lower canopy.

Good for arid mountains, dry pine slopes, chaparral-conifer transitions, high desert woodland, rocky ridges, and sparse evergreen pockets.

```rust
pub enum AridConiferSaplingCell {
    DryFriendSapling(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.24..0.88,
            steepness: 0.0..0.76,
        },
        item: FriendsConifer {
            height: 2.0..4.0,
            canopy_density: Sparse,
            stick_palette_mix: [[dry_conifer_bark..tan_bark], [gray_brown..sun_baked_bark]],
            canopy_palette_mix: [[sage_green..dusty_green], [deep_green..olive_green]],
        },
    }),
    DryNorthernSapling(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.28..0.92,
            steepness: 0.0..0.82,
        },
        item: NorthernConifer {
            height: 2.0..4.0,
            canopy_density: Sparse,
            stick_palette_mix: [[dry_gray_bark..dark_bark], [tan_bark..conifer_bark]],
            canopy_palette_mix: [[blue_sage..dusty_green], [dark_green..olive_green]],
        },
    }),
}

impl CellGrove for AridConiferSapling {
    type Cell = AridConiferSaplingCell;

    const CELL_SIZE_RANGE: Range<f32> = 9.0..18.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.34;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.010..0.044;
}
```

## Construction

* Use low-density placement, roughly `8%-24%`.
* Use Friend's Conifer and Northern Conifer evenly at `2m-4m`.
* Keep both variants common within the sparse distribution; the low density, not the bucket weight, should make them occasional on the terrain.
* Use dry bark, dusty needle, sage, and olive palettes rather than lush evergreen greens.
* Allow steeper slopes than the humid conifer sapling grove, but bias placement toward upland and dry transition bands.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use on dry ridges, exposed slopes, sparse pine woodland, and chaparral-conifer transition zones.
* Pair with Jerry's Chaparral, Levantine Scrub, Spotty Bushes, Wild Grass, exposed rock, and dry ground cover.
* Works well where the scene needs small conifers without committing to a dense evergreen forest.
* Avoid wet jungle, swamp, or lush temperate interiors where the dry palette and sparse spacing would feel out of place.
