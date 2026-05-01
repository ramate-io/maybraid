# 3.5.4.4: Liam's Summer

Liam's Summer is a tropical-toned layering over monster and braid grass. It strongly favors Monster Grass in the understory, keeps Braid Grass present, and uses warm sparse-canopy surprises above palms.

```rust
pub struct LiamsSummer {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 4.0),
            (Allbed, 1.0),
            (FleckingBed, 1.0),
            (GrassyMounds, 0.75),
        ],
        flop: [
            (None, 3.0),
            (FleckingBed, 1.0),
            (FloorScrub, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 1.0),
        (WildGrass, 1.5),
        (TropicalTufts, 1.5),
        (TallGrass, 0.75),
    ],
    understory: UnderstoryLayer [
        (None, 0.75),
        (MonsterGrass, 5.0),
        (BraidGrass, 1.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 2.0),
        (UnendingJungle, 0.75),
        (StrangeOasis, 0.5),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 3.0),
        (PalmShade, 1.5),
        (ForlornSavanna, 0.5),
        (WanderingAcacia, 0.5),
        (JungleMassives, 0.1),
        (TemperateMassives, 0.1),
    ],
}
```

## Intent

Use Liam's Summer for bright, hot, tropical-adjacent fields and groves where the ground is animated by large grasses and the upper canopy appears in occasional bold patches.

## Compatibility Notes

* Monster Grass should be the most likely understory.
* Palm Shade is the main upper-canopy option, but still only moderate.
* Forlorn Savanna and Wandering Acacia add dry-summer interruptions.
* Jungle Massives and Temperate Massives are very rare scenic surprises.
