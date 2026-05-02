# 3.5.4.5: Owl's Desert

Owl's Desert is a sparse desert layering with a high chance of `None` across all layers. Vegetation appears as isolated scrub, dry oasis pockets, or wandering acacia-like trees.

```rust
pub struct OwlsDesert {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 16.0),
            (FloorScrub, 1.0),
        ],
        flop: [
            (None, 10.0),
            (FleckingBed, 0.25),
            (GrassyMounds, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 16.0),
        (BushScrub, 1.0),
        (WildGrass, 0.35),
        (CommonTufts, 0.25),
    ],
    understory: UnderstoryLayer [
        (None, 16.0),
        (LevantineScrub, 1.0),
        (SpottyBushes, 0.5),
        (LowBush, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 32.0),
        (StrangeOasis, 1.0),
        (AridConiferSapling, 0.35),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 8.0),
        (WanderingAcacia, 1.0),
        (Dryland, 0.5),
        (PalmShade, 0.25),
    ],
}
```

## Intent

Use Owl's Desert for mostly open arid terrain. Vegetation should feel discovered rather than continuous.

## Compatibility Notes

* `None` dominates every layer.
* Strange Oasis provides rare lower-canopy pockets.
* Wandering Acacia provides rare upper-canopy presence.
* Levantine Scrub is the main understory exception.
