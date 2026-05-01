# 3.4.6.2: Strange Oasis

Strange Oasis is a low-density lower-canopy grove built around compact [Date Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-09-date-palm/README.md), rare [Penmarch Torch](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-04-penmarch-torch/README.md), and less common [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) variants.

It should feel like a planted or uncanny wet pocket in dry terrain: palms provide the main identity, rare torch forms add strange vertical punctuation, and occasional rounded Storybook trees soften the oasis edge.

Good for desert springs, strange gardens, canyon water pockets, magical groves, oasis settlements, and warm lowland transitions.

```rust
pub enum StrangeOasisCell {
    CompactDatePalm(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.38,
            steepness: 0.0..0.28,
        },
        item: DatePalm {
            height: 3.0..5.0,
            crown_density: Moderate,
            palette_mix: [[deep_green..fresh_green], [yellow_green..date_green]],
        },
    }),
    TorchAccent(Bucket {
        weight: 0.30,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.34,
        },
        item: PenmarchTorch {
            height: 3.0..7.0,
            canopy_density: Sparse..Moderate,
            palette_mix: [[dark_green..olive_green], [flower_yellow..fresh_green]],
        },
    }),
    OasisStorybook(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.42,
            steepness: 0.0..0.32,
        },
        item: StorybookTree {
            height: 4.0..6.0,
            canopy_density: Sparse..Moderate,
            palette_mix: [[green..light_green], [olive_green..fresh_green]],
        },
    }),
}

impl CellGrove for StrangeOasis {
    type Cell = StrangeOasisCell;

    const CELL_SIZE_RANGE: Range<f32> = 8.0..16.0;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.25;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.012..0.050;
}
```

## Construction

* Use low-density placement, roughly `8%–24%`.
* Make Date Palm variants common and keep them compact at `3m–5m`.
* Use Storybook variants less commonly at `4m–6m`.
* Use Penmarch Torch variants rarely at `3m–7m`.
* Keep elevation constraints low, so the grove prefers oasis floors, washes, and sheltered wet pockets.
* Use dry-to-wet contrast palettes: deep palm greens, olive greens, pale flowering accents, and bright oasis growth.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where a sparse lower canopy should imply water, cultivation, or strangeness in dry terrain.
* Pair with Riverine Green, Floor Scrub, exposed sand or stone, pools, reeds, and date-bearing upper layers.
* Works well as a landmark grove around springs and canyon basins.
* Avoid dense placement; the oasis should remain open enough to read as a pocket.
