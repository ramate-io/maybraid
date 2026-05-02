# 3.5.4.15: Bush

Bush is a dry brush-and-open-tree layering. Low Bush and High Bush are common; Forlorn Savanna and Wandering Acacia are common upper-canopy choices; Goettingen Follow is less common; Braid Grass and Monster Grass are less common; Storyteller's is rare.

```rust
pub struct Bush {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 2.0),
            (FloorScrub, 1.0),
            (Allbed, 0.75),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (BushScrub, 1.0),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 1.0),
        (LowBush, 2.0),
        (HighBush, 2.0),
        (BraidGrass, 0.75),
        (MonsterGrass, 0.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 2.5),
        (GoettingenFollow, 0.75),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.5),
        (ForlornSavanna, 1.5),
        (WanderingAcacia, 1.5),
        (Storytellers, 0.20),
    ],
}
```

## Intent

Use Bush for warm open shrublands with enough upper-canopy punctuation to keep the horizon lively.
