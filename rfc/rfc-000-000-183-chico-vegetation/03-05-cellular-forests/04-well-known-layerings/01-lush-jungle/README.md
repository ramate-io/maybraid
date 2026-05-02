# 3.5.4.1: Lush Jungle

Lush Jungle is a dense tropical layering based on the original forest-layer example. It combines wet ground cover, tropical tufts, heavy understory, substantial lower canopy, and a tropical upper canopy with rare jungle massive accents.

```rust
pub struct LushJungle {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (HuelgoatPitch, 1.0),
            (FleckingBed, 1.0),
            (Allbed, 2.0),
        ],
        flop: [
            (None, 4.0),
            (GrassyMounds, 1.0),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (TallGrass, 1.0),
        (WildGrass, 1.0),
        (TropicalTufts, 1.0),
    ],
    understory: UnderstoryLayer [
        (None, 1.0),
        (BraidGrass, 0.5),
        (MonsterGrass, 0.1),
        (TropicalUndergrowth, 1.0),
        (TropicalThicket, 1.0),
        (SpottyBushes, 1.0),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 2.0),
        (UnendingJungle, 2.0),
        (Shamanhome, 0.5),
        (JungleLowerMassives, 0.2),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 2.0),
        (TradeWinds, 4.0),
        (PalmShade, 2.0),
        (RiparianGeneral, 2.0),
        (Leeward, 1.0),
        (JungleMassives, 0.2),
    ],
}
```

## Intent

Use Lush Jungle for wet, layered tropical forest. It should often produce multiple active layers, with `None` still present so gaps, paths, and small clearings can appear.

## Compatibility Notes

* The lower canopy strongly favors Unending Jungle, with rare lower massive structure.
* The upper canopy favors Trade Winds and palms rather than giant canopy every time.
* Ground cover and understory should feel wet and crowded, but not uniformly full.
