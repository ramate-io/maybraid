# 3.4.5.7: Jerry's Chaparral

Jerry's Chaparral is a moderately dense dry understory grove using [Rory's Head-trained](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md), [High Bush](../04-high-bush/README.md), and small [Friend's Conifer](../../../03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md) constructions.

It represents a scrubby chaparral layer with trained, flattened crowns, rounded woody bushes, and occasional small conifer accents. The grove should feel tough, dry, and wind-shaped rather than lush.

Good for chaparral hillsides, dry woodland edges, rocky uplands, coastal scrub, fire-adapted regrowth, and sparse transitional forests.

```rust
pub enum JerrysChaparralCell {
    DryRoryHeadTrained(Bucket {
        weight: 1.5,
        placement_constraints: PlacementConstraints {
            elevation: 0.10..0.65,
            steepness: 0.0..0.78,
        },
        item: RoryHeadTrained {
            height: 1.20..3.20,
            stalk_radius: 0.030,
            canopy_spread: 0.80..2.00,
            canopy_density: Sparse..Moderate,
            stick_palette_mix: [
                [dry_bark..gray_brown],
                [vine_bark..tan_brown],
            ],
            canopy_palette_mix: [
                [olive_green..dry_green],
                [scrub_green..pale_green],
                [dark_green..yellow_green],
            ],
        },
    }),
    ChaparralHighBush(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.70,
            steepness: 0.0..0.55,
        },
        item: CommonHighBush {
            height: 1.00..2.40,
            shoot_count: 7..=11,
            projection_count: Moderate,
            branching: 2..=4,
            leaf_radius: 0.05..0.11,
            stick_palette_mix: [
                [dry_bark..tan_brown],
                [gray_brown..straw_brown],
            ],
            canopy_palette_mix: [
                [dry_green..olive_green],
                [scrub_green..tan_green],
                [dark_green..pale_green],
            ],
        },
    }),
    SmallFriendsConifer(Bucket {
        weight: 0.45,
        placement_constraints: PlacementConstraints {
            elevation: 0.15..0.75,
            steepness: 0.0..0.65,
        },
        item: FriendsConifer {
            height: 2.00..6.00,
            stalk_radius: 0.025,
            canopy_spread: 0.50..1.40,
            canopy_density: Sparse..Moderate,
            stick_palette_mix: [
                [conifer_bark..dark_bark],
                [gray_brown..dry_bark],
            ],
            canopy_palette_mix: [
                [dark_green..blue_green],
                [dry_green..deep_green],
                [olive_green..needle_green],
            ],
        },
    }),
    ManzanitaRory(Bucket {
        weight: 0.35,
        placement_constraints: PlacementConstraints {
            elevation: 0.15..0.70,
            steepness: 0.0..0.72,
        },
        item: RoryHeadTrained {
            height: 1.40..3.00,
            stalk_radius: 0.030,
            canopy_spread: 0.90..2.10,
            canopy_density: Sparse,
            stick_palette_mix: [
                [manzanita_red..copper_red],
                [smooth_burgundy..orange_bark],
            ],
            canopy_palette_mix: [
                [olive_green..pale_green],
                [flower_white..dry_green],
                [dark_green..yellow_green],
            ],
        },
    }),
}

impl CellGrove for JerrysChaparral {
    type Cell = JerrysChaparralCell;

    const CELL_SIZE_RANGE: Range<f32> = 4.0..9.0;
    const DENSITY_RANGE: Range<f32> = 0.24..0.52;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.35;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.02..0.08;
}
```

## Construction

* Use moderate placement, roughly `24%–52%`.
* Use Rory's Head-trained forms as dry, flattened crown accents.
* Use High Bush variants as the primary chaparral mass.
* Add small Friend's Conifer variants rarely; keep them below `6m`.
* Add rare red-barked trained forms for Manzanita-like chaparral color pops.
* Prefer dry palettes: olive, scrub green, pale green, tan-green, and dark blue-green conifer accents.
* Let variants tolerate more slope than lush understory groves, but keep tree-like variants slightly stricter than bushes.
* Use deterministic yaw, scale, canopy spread, branch density, and conifer height sampling.
* Use [Bucket Throw](../../02-selection-and-placement/01-bucket-throw/README.md) varietal selection.

## Usage

* Use where dry understory should read as woody, open, and fire-adapted.
* Pair with [Floor Scrub](../../03-well-known-ground-cover-groves/04-floor-scrub/README.md), exposed rock, dry grass, and sparse trees.
* Works well on coastal slopes, rocky paths, dry ridges, and scrubby forest edges.
* Keep conifers occasional, so the grove remains chaparral, not young conifer woodland.
* Avoid lush palettes or continuous coverage; gaps and dry exposed terrain should remain visible.
