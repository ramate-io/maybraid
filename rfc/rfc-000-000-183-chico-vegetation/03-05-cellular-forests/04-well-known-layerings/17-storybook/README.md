# 3.5.4.17: Storybook

Storybook is a whimsical, mixed layering. Riparian Mix is common, while Storyteller's, Leeward, and Trade Winds are less common. Huelgoat Pitch is common ground cover. Low Bush and High Bush are rare, and Riverine Green is less common.

```rust
pub struct Storybook {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (HuelgoatPitch, 2.0),
            (FleckingBed, 0.75),
        ],
        flop: [
            (None, 4.0),
            (Allbed, 0.5),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (WildGrass, 1.0),
        (CommonTufts, 0.75),
    ],
    understory: UnderstoryLayer [
        (None, 2.0),
        (RiverineGreen, 0.75),
        (LowBush, 0.25),
        (HighBush, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 3.0),
        (GoettingenFollow, 0.5),
        (UnendingJungle, 0.25),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (RiparianMix, 2.0),
        (Storytellers, 0.75),
        (Leeward, 0.75),
        (TradeWinds, 0.75),
    ],
}
```

## Intent

Use Storybook for bright, varied fantasy forest edges where riparian structure anchors more whimsical canopy options.
