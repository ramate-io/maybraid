# 3.5.4.7: Seceda

Seceda is an alpine scrub-conifer layering. Jerry's Chaparral, High Bush, Arid Conifer Sapling, Alpine, and Allbed are common. Dryland, Christmas Taiga, Conifer Massives, and Conifer Lower Massives are rare.

```rust
pub struct Seceda {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (Allbed, 3.0),
            (HuelgoatPitch, 0.75),
        ],
        flop: [
            (None, 4.0),
            (FloorScrub, 0.75),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.5),
        (CommonTufts, 1.0),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 1.5),
        (JerrysChaparral, 2.0),
        (HighBush, 2.0),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 1.5),
        (AridConiferSapling, 2.0),
        (ConiferLowerMassives, 0.25),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (Alpine, 2.0),
        (Dryland, 0.35),
        (ChristmasTaiga, 0.35),
        (ConiferMassives, 0.25),
    ],
}
```

## Intent

Use Seceda for exposed alpine meadow and scrub slopes with conifer structure nearby but not everywhere.
