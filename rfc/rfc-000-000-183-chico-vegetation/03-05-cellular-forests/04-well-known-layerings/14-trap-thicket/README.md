# 3.5.4.14: Trap Thicket

Trap Thicket is a dense tropical layering. Unending Jungle is common in the lower canopy, while Monster Grass, Braid Grass, Tropical Thicket, and Tropical Undergrowth are all common in the understory.

```rust
pub struct TrapThicket {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.5),
            (Allbed, 1.5),
            (HuelgoatPitch, 0.75),
        ],
        flop: [
            (None, 4.0),
            (FleckingBed, 0.5),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (TropicalTufts, 1.0),
        (WildGrass, 0.75),
    ],
    understory: UnderstoryLayer [
        (None, 0.75),
        (MonsterGrass, 1.5),
        (BraidGrass, 1.5),
        (TropicalThicket, 1.5),
        (TropicalUndergrowth, 1.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 1.0),
        (UnendingJungle, 2.5),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 2.0),
        (TradeWinds, 0.75),
        (JungleMassives, 0.20),
    ],
}
```

## Intent

Use Trap Thicket for oppressive, hard-to-cross jungle where the understory is the main obstacle.
