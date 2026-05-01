# 3.5.4.18: Meadowland

Meadowland is an open layering with common Huelgoat Pitch ground cover. Most layers are empty. Riparian Mix and Rolling Oaks are rare, and Temperate, Jungle, and Conifer Massives are very rare.

```rust
pub struct Meadowland {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (HuelgoatPitch, 2.0),
            (Allbed, 0.75),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 3.0),
        (WildGrass, 1.0),
        (TallGrass, 0.75),
    ],
    understory: UnderstoryLayer [
        (None, 8.0),
        (LowBush, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 9.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 8.0),
        (RiparianMix, 0.5),
        (RollingOaks, 0.5),
        (TemperateMassives, 0.15),
        (ConiferMassives, 0.10),
    ],
}
```

## Intent

Use Meadowland for mostly open, soft terrain where canopy appears as rare edge, landmark, or distant-tree events.
