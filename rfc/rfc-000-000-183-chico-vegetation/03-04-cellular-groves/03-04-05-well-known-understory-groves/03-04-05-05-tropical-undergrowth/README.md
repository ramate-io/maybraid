# 3.4.5.5: Tropical Undergrowth

Tropical Undergrowth is a lush tropical understory grove using [Tuft](../../../03-01-stalk-and-ball-stick-trees/03-01-02-ball-components/03-01-02-06-tufts/README.md), small [Palm Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-10-palm-bush/README.md), mini [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-07-rory-s-head-trained/README.md), rare mini [Vase Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-03-vase-tree/README.md), and rare mini [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) constructions.

It represents mixed tropical growth where the ground layer begins to feel like a young jungle: tufted greens fill the gaps, juvenile palm-like forms add broad leaves, and small trained tree forms create horizontal and vase-like foliage accents without becoming a true lower canopy.

Good for jungle margins, humid forest paths, riverbanks, coastal lowlands, canopy gaps, old clearings, and dense tropical transitions beneath larger trees.

```rust
pub enum TropicalUndergrowthCell {
    BrightTuft(Bucket {
        weight: 2.0,
        item: Tuft {
            height: 0.30..0.70,
            width: 0.16..0.42,
            palette_mix: [
                [bright_green..lime_green],
                [lush_green..fresh_green],
                [yellow_green..light_green],
            ],
        },
    }),
    DeepTuft(Bucket {
        weight: 1.5,
        item: Tuft {
            height: 0.40..0.90,
            width: 0.18..0.50,
            palette_mix: [
                [deep_green..emerald_green],
                [dark_green..wet_green],
                [blue_green..bright_green],
            ],
        },
    }),
    SmallPalmBush(Bucket {
        weight: 1.0,
        item: PalmBush {
            height: 0.50..1.40,
            frond_count: 5..=9,
            frond_length: 0.25..0.70,
            crown_spread: 0.35..0.90,
            palette_mix: [
                [lush_green..bright_green],
                [deep_green..fresh_green],
                [wet_green..lime_green],
            ],
        },
    }),
    MiniRoryHeadTrained(Bucket {
        weight: 0.85,
        item: RoryHeadTrained {
            height: 0.80..1.80,
            stalk_radius: 0.025,
            canopy_spread: 0.50..1.20,
            canopy_density: Sparse..Moderate,
            palette_mix: [
                [deep_green..fresh_green],
                [blue_green..wet_green],
                [yellow_green..lime_green],
            ],
        },
    }),
    MiniVaseTree(Bucket {
        weight: 0.20,
        item: VaseTree {
            height: 1.00..2.30,
            stalk_radius: 0.030,
            canopy_spread: 0.70..1.50,
            canopy_density: Sparse,
            palette_mix: [
                [lush_green..bright_green],
                [dark_green..emerald_green],
                [flower_white..fresh_green],
            ],
        },
    }),
    MiniSparseStorybook(Bucket {
        weight: 0.15,
        item: StorybookTree {
            height: 1.20..2.50,
            stalk_radius: 0.030,
            canopy_spread: 0.60..1.40,
            canopy_density: Sparse,
            palette_mix: [
                [deep_green..light_green],
                [wet_green..fresh_green],
                [blue_green..yellow_green],
            ],
        },
    }),
}

impl CellGrove for TropicalUndergrowth {
    type Cell = TropicalUndergrowthCell;

    const CELL_SIZE_RANGE: Range<f32> = 3.0..7.0;
    const DENSITY_RANGE: Range<f32> = 0.22..0.58;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.70;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.30;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.12..0.38;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.03..0.10;
}
```

## Construction

* Use moderate to dense placement, roughly `22%–58%`.
* Use tropical tufts and small Palm Bush forms as the common visual base.
* Commonly add mini Rory's Head-trained forms around `80cm–1.8m` to introduce trained horizontal foliage planes.
* Rarely add mini Vase Tree forms around `1m–2.3m` for upward-opening ornamental or jungle-cup silhouettes.
* Rarely add mini Storybook Tree forms around `1.2m–2.5m`, but force sparse canopy allocation so they read as juvenile understory, not full trees.
* Keep all tree-like variants subordinate to the grove layer; they should add structure, not become canopy.
* Use wet, saturated palettes: bright green, lime, blue-green, emerald, young yellow-green, and occasional flowering accents.
* Use deterministic yaw, scale, frond count, canopy spread, and sparse-canopy sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where tropical understory should feel mixed, layered, and humid without becoming a full lower-canopy grove.
* Pair with [Tropical Tufts](../../03-04-04-well-known-tufts-groves/03-04-04-05-tropical-tufts/README.md), [Monster Grass](../03-04-05-02-monster-grass/README.md), wet ground cover, vines, palms, and dense jungle trees.
* Works well along jungle trails, riverbanks, canopy breaks, coastal forest edges, and around young tropical regrowth.
* Keep mini tree forms rare enough that the grove still reads as understory.
* Avoid using alone for mature jungle density; it should be the mixed mid-layer between ground vegetation and larger trees.
