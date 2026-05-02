# 3.5.4.3: Taiga

Taiga mixes alpine, scrubland, and Christmas Taiga structure. It generally has minimal understory, with most of the identity carried by conifer canopy, sparse ground cover, and cold tuft texture.

```rust
pub struct Taiga {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.5),
            (HuelgoatPitch, 1.0),
            (Allbed, 1.0),
            (FloorScrub, 0.5),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (CommonTufts, 1.0),
        (TallGrass, 0.75),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 5.0),
        (JerrysChaparral, 0.5),
        (SpottyBushes, 0.5),
        (LowBush, 0.35),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 2.0),
        (ConiferSapling, 1.5),
        (AridConiferSapling, 0.75),
        (ConiferLowerMassives, 0.25),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (ChristmasTaiga, 2.0),
        (Alpine, 1.5),
        (ConiferMassives, 0.01),
        (Dryland, 0.25),
    ],
}
```

## Intent

Use Taiga for cold conifer forest with open walking space and restrained lower growth. It should not feel like a dense brush forest.

## Compatibility Notes

* Understory is intentionally mostly `None`.
* Christmas Taiga and Alpine define the main canopy.
* Scrubland influence comes through Floor Scrub, Jerry's Chaparral, Spotty Bushes, and sparse dry conifer options.
