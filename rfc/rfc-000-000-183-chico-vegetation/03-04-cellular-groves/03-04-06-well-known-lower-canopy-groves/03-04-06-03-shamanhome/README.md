# 3.4.6.3: Shamanhome

Shamanhome is a moderate-density lower-canopy grove mixing less common [Date Palm](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-09-date-palm/README.md), less common [Sope's Banyan](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-06-sope-s-banyan/README.md), and common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md) variants.

It should feel like a small sacred or lived-in grove: braided oak structure dominates, while palms and banyan forms appear as uncommon vertical and mystical accents. The result is denser and more intentional than wild woodland, but not uniform enough to read as an orchard.

Good for ritual clearings, old villages, magical groves, sheltered tropical-temperate transitions, and areas where lower canopy should feel culturally or spiritually marked.

```rust
pub enum ShamanhomeCell {
    ShamanBraidOak(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.62,
            steepness: 0.0..0.40,
        },
        item: BraidOak {
            height: 4.0..7.0,
            canopy_density: Moderate,
            stick_palette_mix: [[dark_bark..moss_bark], [gnarled_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..fresh_green], [moss_green..light_green]],
        },
    }),
    RedRitualBraidOak(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.58,
            steepness: 0.0..0.45,
        },
        item: BraidOak {
            height: 4.0..7.0,
            canopy_density: Moderate,
            stick_palette_mix: [[ritual_red_bark..copper_red], [dark_bark..moss_bark]],
            canopy_palette_mix: [[deep_green..fresh_green], [flower_red..moss_green]],
        },
    }),
    RitualDatePalm(Bucket {
        weight: 0.75,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.30,
        },
        item: DatePalm {
            height: 4.0..6.0,
            crown_density: Moderate,
            stick_palette_mix: [[palm_bark..tan_bark], [dry_brown..gray_brown]],
            canopy_palette_mix: [[deep_green..date_green], [yellow_green..fresh_green]],
        },
    }),
    SmallSopeBanyan(Bucket {
        weight: 0.80,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.55,
            steepness: 0.0..0.36,
        },
        item: SopesBanyan {
            height: 5.0..7.0,
            canopy_density: Moderate,
            descender_frequency: Sparse,
            stick_palette_mix: [[banyan_bark..dark_bark], [wet_brown..gray_brown]],
            canopy_palette_mix: [[dark_green..wet_green], [blue_green..deep_green]],
        },
    }),
}

impl CellGrove for Shamanhome {
    type Cell = ShamanhomeCell;

    const CELL_SIZE_RANGE: Range<f32> = 7.0..14.0;
    const DENSITY_RANGE: Range<f32> = 0.22..0.48;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.012..0.050;
}
```

## Construction

* Use moderate placement, roughly `22%–48%`.
* Make Braid Oak common at `4m–7m`; it should define the grove's structure.
* Use Date Palm less commonly at `4m–6m`.
* Use Sope's Banyan less commonly at `5m–7m`.
* Include rare red-barked Braid Oak variants for ritual color punctuation.
* Keep banyan descenders sparse so lower canopy does not become full banyan forest.
* Prefer deep greens, mossy bark, and occasional ritual or cultivated palette accents.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where lower canopy should feel curated, mystical, or inhabited.
* Pair with Tropical Thicket, Riverine Green, old paths, stones, shrines, roots, and dense ground cover.
* Works well as a distinctive grove around settlement-like or ritual spaces.
* Avoid overusing palms; Braid Oak should remain the common visual anchor.
