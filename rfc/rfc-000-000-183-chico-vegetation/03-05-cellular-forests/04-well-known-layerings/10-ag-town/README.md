# 3.5.4.10: Ag Town

Ag Town is a cultivated settlement-edge layering. Orchard, Vineyard, and Date Grove are common, while other layers are mostly empty and ground cover is only rarely populated.

```rust
pub struct AgTown {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 6.0),
            (Allbed, 0.75),
            (FleckingBed, 0.5),
        ],
        flop: [
            (None, 10.0),
            (GrassyMounds, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 8.0),
        (CommonTufts, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 8.0),
        (LowBush, 0.35),
        (BraidGrass, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 8.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (Orchard, 2.0),
        (Vineyard, 2.0),
        (DateGrove, 2.0),
    ],
}
```

## Intent

Use Ag Town for agricultural margins where planted tree rows dominate and the unmanaged vegetation layers are mostly suppressed.
