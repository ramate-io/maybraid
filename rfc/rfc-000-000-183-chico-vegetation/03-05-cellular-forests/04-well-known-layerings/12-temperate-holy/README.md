# 3.5.4.12: Temperate Holy

Temperate Holy is a sparse sacred temperate layering. Temperate Lower Massives are common, Temperate Massives are less common, Conifer Massives are rare, and most other layers are empty.

```rust
pub struct TemperateHoly {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 5.0),
            (HuelgoatPitch, 0.75),
            (Allbed, 0.5),
        ],
        flop: [
            (None, 8.0),
            (FleckingBed, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 8.0),
        (CommonTufts, 0.25),
    ],
    understory: UnderstoryLayer [
        (None, 8.0),
        (LowBush, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 1.0),
        (TemperateLowerMassives, 2.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 2.0),
        (TemperateMassives, 1.0),
        (ConiferMassives, 0.25),
    ],
}
```

## Intent

Use Temperate Holy for quiet giant-tree spaces where the vertical mass is sacred and the forest floor stays open.
