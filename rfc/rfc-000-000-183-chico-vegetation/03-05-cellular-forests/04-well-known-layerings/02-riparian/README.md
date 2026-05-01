# 3.5.4.2: Riparian

Riparian is a basic river-corridor layering. It uses wet ground cover, moderate tufts, green understory, sparse lower-canopy fill, and mixed riparian upper canopy.

```rust
pub struct Riparian {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (HuelgoatPitch, 1.0),
            (FleckingBed, 1.0),
            (Allbed, 1.5),
        ],
        flop: [
            (None, 3.0),
            (GrassyMounds, 1.0),
            (FleckingBed, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 1.5),
        (TallGrass, 1.0),
        (WildGrass, 1.0),
        (CommonTufts, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 1.5),
        (RiverineGreen, 2.0),
        (LowBush, 0.75),
        (HighBush, 0.5),
        (SpottyBushes, 0.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 2.0),
        (GoettingenFollow, 1.0),
        (UnendingJungle, 0.35),
        (StrangeOasis, 0.25),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.0),
        (RiparianGeneral, 2.0),
        (RiparianMix, 1.5),
        (PalmShade, 0.35),
        (RollingOaks, 0.35),
    ],
}
```

## Intent

Use Riparian along rivers, streams, wet gullies, and floodplain edges. It should feel green and layered without becoming fully tropical by default.

## Compatibility Notes

* Riverine Green is the primary understory grove.
* Riparian General and Riparian Mix provide the main upper-canopy identity.
* Palm and oasis-like forms are allowed, but kept secondary.
