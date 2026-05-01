# 3.5.4.16: Old Nevada

Old Nevada is a sparse dry conifer layering. Arid Conifer Sapling and Grassy Mounds are common, while most other layers are empty.

```rust
pub struct OldNevada {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.5),
            (GrassyMounds, 2.0),
            (FloorScrub, 0.75),
        ],
        flop: [
            (None, 6.0),
            (FleckingBed, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 5.0),
        (BushScrub, 0.5),
        (WildGrass, 0.35),
    ],
    understory: UnderstoryLayer [
        (None, 8.0),
        (SpottyBushes, 0.35),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 1.5),
        (AridConiferSapling, 2.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 7.0),
        (Dryland, 0.35),
    ],
}
```

## Intent

Use Old Nevada for dry, open terrain with small arid conifers scattered over mounded ground.
