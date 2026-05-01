# 3.4.6.5: Conifer Sapling

Conifer Sapling is a moderate-density lower-canopy grove using common young [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-14-friend-s-conifer/README.md) and [Northern Conifer](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-11-northern-conifer/README.md) variants at `4m-6m`.

It represents young conifer growth beneath taller evergreen stands: dense enough to read as a regenerating forest layer, but low enough that mature upper-canopy trees still dominate the scene.

Good for montane forests, boreal edges, evergreen understories, sheltered ravines, young forest patches, and cool lower-canopy fill beneath older conifers.

```rust
pub enum ConiferSaplingCell {
    FriendSapling(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.18..0.82,
            steepness: 0.0..0.64,
        },
        item: FriendsConifer {
            height: 4.0..6.0,
            canopy_density: Moderate,
            stick_palette_mix: [[conifer_bark..dark_bark], [gray_brown..moss_bark]],
            canopy_palette_mix: [[deep_green..blue_green], [dark_green..fresh_green]],
        },
    }),
    NorthernSapling(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.22..0.88,
            steepness: 0.0..0.72,
        },
        item: NorthernConifer {
            height: 4.0..6.0,
            canopy_density: Moderate,
            stick_palette_mix: [[cold_bark..dark_bark], [gray_brown..conifer_bark]],
            canopy_palette_mix: [[cold_green..blue_green], [deep_green..dark_green]],
        },
    }),
}

impl CellGrove for ConiferSapling {
    type Cell = ConiferSaplingCell;

    const CELL_SIZE_RANGE: Range<f32> = 7.0..14.0;
    const DENSITY_RANGE: Range<f32> = 0.28..0.48;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.014..0.055;
}
```

## Construction

* Use moderate-density placement, roughly `28%-48%`.
* Use Friend's Conifer and Northern Conifer evenly at `4m-6m`.
* Keep both variants common, so the grove reads as a mixed young conifer stand rather than a specialty accent.
* Use cool bark and needle palettes, with Friend's Conifer slightly softer and Northern Conifer slightly colder.
* Let the variants tolerate moderate-to-high slope, but keep elevation biased toward cooler or upland bands.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use beneath taller conifers, elder evergreens, and mountain forest canopies.
* Pair with Huelgoat Pitch, Allbed, Wild Grass, low shrubs, rocks, moss, and fallen logs.
* Works well as a regeneration layer after clearing, fire, windfall, or along forest edges.
* Avoid tropical, desert, or broadleaf-heavy contexts unless the biome deliberately wants an odd evergreen pocket.
