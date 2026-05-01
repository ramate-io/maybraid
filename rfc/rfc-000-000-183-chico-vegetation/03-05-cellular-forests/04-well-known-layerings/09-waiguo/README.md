# 3.5.4.9: Waiguo

Waiguo is a cultivated mixed grove layering. Orchard, Vineyard, and Date Grove are common; Braid Grass is common below them; Tropical Thicket is less common; Temperate Massives are rare.

```rust
pub struct Waiguo {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 2.0),
            (Allbed, 1.0),
            (FleckingBed, 0.75),
        ],
        flop: [
            (None, 5.0),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (CommonTufts, 1.0),
        (TallGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 1.5),
        (BraidGrass, 2.0),
        (TropicalThicket, 0.75),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 4.0),
        (GoettingenFollow, 0.5),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (Orchard, 2.0),
        (Vineyard, 2.0),
        (DateGrove, 2.0),
        (TemperateMassives, 0.20),
    ],
}
```

## Intent

Use Waiguo for productive, cultivated, warm-climate groves where managed trees coexist with lush but controlled understory.
