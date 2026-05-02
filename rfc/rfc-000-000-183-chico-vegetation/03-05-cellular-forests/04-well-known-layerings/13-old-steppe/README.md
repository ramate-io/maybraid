# 3.5.4.13: Old Steppe

Old Steppe is an open grassland layering. Allbed and Grassy Mounds are common, most other layers are empty, and Conifer Sapling appears rarely.

```rust
pub struct OldSteppe {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (Allbed, 2.0),
            (GrassyMounds, 2.0),
        ],
        flop: [
            (None, 4.0),
            (FleckingBed, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 4.0),
        (TallGrass, 0.75),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 8.0),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 8.0),
        (ConiferSapling, 0.35),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 10.0),
    ],
}
```

## Intent

Use Old Steppe for broad, old grasslands where trees are unusual, and the ground plane carries the scene.
