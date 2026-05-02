# 3.5.4.21: Open Tropics

Open Tropics is a sparse tropical layering with common Huelgoat Pitch and less common Trade Winds canopy.

```rust
pub struct OpenTropics {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.5),
            (HuelgoatPitch, 2.0),
            (Allbed, 0.5),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
            (GrassyMounds, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 4.0),
        (TropicalTufts, 0.75),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 6.0),
        (TropicalUndergrowth, 0.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 7.0),
        (UnendingJungle, 0.35),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 3.0),
        (TradeWinds, 1.0),
    ],
}
```

## Intent

Use Open Tropics for warm open terrain where tropical ground character is present, but tree canopy appears only sometimes.
