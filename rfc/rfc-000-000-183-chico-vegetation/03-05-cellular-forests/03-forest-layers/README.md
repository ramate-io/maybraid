# 3.5.2: Forest Layers

Forest cells are composed of:

1. Ground cover layers.
2. Tufts layers. 
3. Understory layers. 
4. Lower canopy layers. 
5. Upper canopy layers. 

...defining compatibility. 

Selection of the particular grove within a layer is given by the [Bucket Throw](../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) algorithm.

A forest cell is thus defined something like:

```rust
pub struct LushJungle {
    ground_cover: GroundCover {
        flip: [
            (None, 1.0),
            (HuelgoatPitch, 1.0),
            (FleckingBed, 1.0),
            (Allbed, 2.0)
        ],
        flop: [
            (None, 4.0),
            (GrassyMounds, 1.0)
        ],
    },
    tufts: [
        (None, 2.0),
        (TallGrass, 1.0),
        (WildGrass, 1.0),
        (TropicalTufts, 1.0),
    ],
    understory: [
        (None, 1.0),
        (BraidGrass, 0.5),
        (MonsterGrass, 0.1),
        (TropicalUndergrowth, 1.0),
        (TropicalThicket, 1.0),
        (SpottyBushes, 1.0)
    ],
    lower_canopy: [
        (None, 2.0),
        (UnendingJungle, 2.0),
        (Shamanhome, 0.5),
        (LowerJungleMassives, 0.2)
    ],
    upper_canopy: [
        (None, 2.0),
        (TradeWinds, 4.0),
        (PalmShade, 2.0),
        (RiparianGeneral, 2.0),
        (Leeward, 1.0)
        (JungleMassives, 0.2)
    ]
}
```


Subsections:
