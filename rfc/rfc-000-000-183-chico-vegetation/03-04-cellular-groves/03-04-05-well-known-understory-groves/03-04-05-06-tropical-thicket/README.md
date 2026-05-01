# 3.4.5.6: Tropical Thicket

Tropical Thicket is a dense tropical understory grove using larger [Palm Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-10-palm-bush/README.md), mini [Honu Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-05-honu-banyan/README.md), and moderate-size [Common High Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-12-common-high-bush/README.md) constructions.

It represents a thicker, woodier tropical mid-layer than [Tropical Undergrowth](../03-04-05-05-tropical-undergrowth/README.md): broad palm leaves provide the main mass, small banyan-like forms add irregular trunk and descender structure, and common high bushes fill the gaps with rounded shrub volume.

Good for jungle edges, riverine thickets, humid ravines, dense coastal forest, young banyan groves, hidden paths, and areas where tropical understory should shape movement and sightlines.

```rust
pub enum TropicalThicketCell {
    LargePalmBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.28,
        },
        item: PalmBush {
            height: 1.00..2.20,
            frond_count: 7..=12,
            frond_length: 0.55..1.30,
            crown_spread: 0.80..1.80,
            stick_palette_mix: [
                [palm_bark..tan_bark],
                [green_stem..wet_brown],
            ],
            canopy_palette_mix: [
                [lush_green..bright_green],
                [deep_green..fresh_green],
                [wet_green..lime_green],
            ],
        },
    }),
    BroadWetPalmBush(Bucket {
        weight: 1.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.68,
        },
        item: PalmBush {
            height: 1.20..2.60,
            frond_count: 8..=14,
            frond_length: 0.70..1.60,
            crown_spread: 1.00..2.20,
            stick_palette_mix: [
                [palm_bark..dark_bark],
                [wet_brown..green_brown],
            ],
            canopy_palette_mix: [
                [blue_green..deep_green],
                [emerald_green..wet_green],
                [yellow_green..fresh_green],
            ],
        },
    }),
    MiniHonuBanyan(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.28,
        },
        item: HonuBanyan {
            height: 1.80..3.80,
            stalk_radius: 0.2,
            canopy_spread: 1.20..2.80,
            descender_frequency: Sparse,
            canopy_density: Sparse..Moderate,
            stick_palette_mix: [
                [banyan_bark..dark_bark],
                [wet_brown..gray_brown],
            ],
            canopy_palette_mix: [
                [dark_green..deep_green],
                [wet_green..blue_green],
                [emerald_green..fresh_green],
            ],
        },
    }),
    ModerateHighBush(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.28,
        },
        item: CommonHighBush {
            height: 1.20..2.40,
            shoot_count: 7..=11,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.06..0.13,
            stick_palette_mix: [
                [shrub_bark..green_brown],
                [dark_bark..wet_brown],
            ],
            canopy_palette_mix: [
                [deep_green..fresh_green],
                [lush_green..bright_green],
                [blue_green..light_green],
            ],
        },
    }),
    FloweringHighBush(Bucket {
        weight: 0.30,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.78,
        },
        item: CommonHighBush {
            height: 1.00..2.20,
            shoot_count: 7..=10,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.06..0.12,
            stick_palette_mix: [
                [shrub_bark..dark_bark],
                [green_brown..wet_brown],
            ],
            canopy_palette_mix: [
                [dark_green..leaf_green],
                [flower_white..fresh_green],
                [flower_yellow..lime_green],
            ],
        },
    }),
    RedStemPalmBush(Bucket {
        weight: 0.25,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.42,
            steepness: 0.0..0.60,
        },
        item: PalmBush {
            height: 1.00..2.30,
            frond_count: 6..=11,
            frond_length: 0.55..1.35,
            crown_spread: 0.80..1.80,
            stick_palette_mix: [
                [red_palm_stem..copper_red],
                [wet_burgundy..dark_bark],
            ],
            canopy_palette_mix: [
                [deep_green..bright_green],
                [lime_green..fresh_green],
                [blue_green..wet_green],
            ],
        },
    }),
}

impl CellGrove for TropicalThicket {
    type Cell = TropicalThicketCell;

    const CELL_SIZE_RANGE: Range<f32> = 4.0..9.0;
    const DENSITY_RANGE: Range<f32> = 0.24..0.62;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.12..0.40;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.02..0.08;
}
```

## Construction

* Use moderate to dense placement, roughly `24%–62%`.
* Use larger Palm Bush variants as the common structural base.
* Use moderate-size Common High Bush forms around `1.2m–2.4m` to fill gaps and round out the thicket.
* Rarely add mini Honu Banyan forms around `1.8m–3.8m`; keep descenders sparse so they read as thicket texture rather than mature banyan structure.
* Include rare red-stem palm bush variants for saturated tropical color accents.
* Let palm fronds overlap enough to create a thick tropical read, but keep enough gaps for navigation and silhouette readability.
* Prefer wet tropical palettes: deep green, blue-green, emerald, lime, fresh yellow-green, and occasional flowering highlights.
* Use deterministic yaw, scale, frond count, canopy spread, descender frequency, and bush branching sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where tropical understory should feel woody, leafy, and movement-shaping.
* Pair with [Tropical Undergrowth](../03-04-05-05-tropical-undergrowth/README.md), [Monster Grass](../03-04-05-02-monster-grass/README.md), wet ground cover, vines, roots, and larger jungle trees.
* Works well as a dense edge along trails, rivers, ruins, canopy gaps, and banyan-dominated regions.
* Keep mini banyan variants rare unless the region is explicitly a young banyan grove.
* Avoid using as mature forest canopy; it should remain a dense tropical understory thicket.
