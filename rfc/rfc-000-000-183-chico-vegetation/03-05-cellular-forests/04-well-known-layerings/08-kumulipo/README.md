# 3.5.4.8: Kumulipo

Kumulipo is a sacred tropical layering with common Shamanhome lower canopy and Palm Shade upper canopy. Tropical Tufts are less common, understory is mostly empty, and Trade Winds, Leeward, Wandering Acacia, and Jungle Massives appear rarely.

```rust
pub struct Kumulipo {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.5),
            (Allbed, 1.0),
            (HuelgoatPitch, 0.75),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 5.0),
        (TropicalTufts, 1.0),
        (WildGrass, 1.0),
        (CommonTufts, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 5.0),
        (TropicalUndergrowth, 0.5),
        (LowBush, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 1.0),
        (Shamanhome, 2.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (PalmShade, 2.0),
        (TradeWinds, 0.35),
        (Leeward, 0.35),
        (WanderingAcacia, 0.25),
        (JungleMassives, 0.20),
    ],
}
```

## Intent

Use Kumulipo where palms and sacred lower-canopy structure should carry the scene without dense understory clutter.
