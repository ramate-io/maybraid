# 3.4.4.5: Tropical Tufts

Tropical Tufts is a sparse tropical ground grove using [Tuft](../../../03-01-stalk-and-ball-stick-trees/03-01-02-ball-components/03-01-02-06-tufts/README.md) and small [Palm Bush](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-10-palm-bush/) constructions.

It represents tall, vibrant tufts with occasional juvenile palm-like structure. The grove should feel humid and tropical, but not dense by itself. Lushness should come from surrounding broadleaf plants, palms, vines, canopy vegetation, or denser tropical understory groves.

Good for jungle clearings, tropical forest edges, coastal lowlands, riverbanks, wet disturbed terrain, and sparse gaps between larger tropical vegetation.

```rust
pub enum TropicalTuftsCell {
    BrightTuft(Bucket {
        weight: 2.0,
        item: Tuft {
            height: 0.25..0.50,
            width: 0.14..0.34,
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
            height: 0.30..0.55,
            width: 0.16..0.38,
            palette_mix: [
                [deep_green..emerald_green],
                [dark_green..wet_green],
                [blue_green..bright_green],
            ],
        },
    }),
    YellowGreenTuft(Bucket {
        weight: 1.0,
        item: Tuft {
            height: 0.25..0.45,
            width: 0.12..0.30,
            palette_mix: [
                [yellow_green..fresh_green],
                [lime_green..light_green],
                [young_green..bright_green],
            ],
        },
    }),
    SmallPalmBush(Bucket {
        weight: 0.75,
        item: PalmBush {
            height: 0.35..0.80,
            frond_count: 4..=7,
            frond_length: 0.18..0.45,
            crown_spread: 0.25..0.55,
            palette_mix: [
                [lush_green..bright_green],
                [deep_green..fresh_green],
                [wet_green..lime_green],
            ],
        },
    }),
    JuvenilePalmBush(Bucket {
        weight: 0.35,
        item: PalmBush {
            height: 0.50..1.10,
            frond_count: 3..=5,
            frond_length: 0.25..0.60,
            crown_spread: 0.30..0.70,
            palette_mix: [
                [young_green..lime_green],
                [fresh_green..light_green],
                [bright_green..yellow_green],
            ],
        },
    }),
}

impl CellGrove for TropicalTufts {
    type Cell = TropicalTuftsCell;

    const CELL_SIZE_RANGE: Range<f32> = 2.0..4.5;
    const DENSITY_RANGE: Range<f32> = 0.08..0.24;

    const ELEVATION_RANGE: Range<f32> = 0.0..0.65;
    const STEEPNESS_RANGE: Range<f32> = 0.0..0.35;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.10..0.30;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.04..0.12;
}
```

## Construction

* Use sparse placement, roughly `8%–24%`.
* Use tall tufts around `25cm–50cm`.
* Use multiple vibrant tropical palette ranges per varietal.
* Prefer varietals that include:
  * bright green
  * lime green
  * deep emerald
  * wet green
  * blue-green
  * yellow-green new growth
* Include occasional small Palm Bush variants, but keep them subordinate to tufts.
* Do not use enough density for this grove to read as full tropical undergrowth by itself.
* Use deterministic yaw and scale sampling.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where tropical ground cover should feel present but not continuous.
* Pair with dense jungle groves, palm groves, vines, exposed wet soil, or riverbank vegetation.
* Works well as spacing material between larger tropical plants.
* Use near water, humid lowlands, tropical paths, and canopy gaps.
* Avoid using alone for lush jungle density; it should read as a sparse tropical accent layer.
