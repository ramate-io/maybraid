# 3.5.3: Forest Layers

This page is subsection **3.5.3** of [RFC-183: Chico Vegetation](../../README.md)

Forest cells are composed of ordered layers of groves. Each layer owns one vertical or ecological band of vegetation, and each forest layering provides a distribution of compatible grove choices for that band.

The standard forest layers are:

1. [Ground Cover Layer](./01-ground-cover-layer/README.md)
2. [Tufts Layer](./02-tufts-layer/README.md)
3. [Understory Layer](./03-understory-layer/README.md)
4. [Lower Canopy Layer](./04-lower-canopy-layer/README.md)
5. [Upper Canopy Layer](./05-upper-canopy-layer/README.md)

A forest layering does not place vegetation directly. Instead, it says which groves are allowed in each layer, how likely each grove is, and whether that layer is allowed to be empty. The actual grove within a layer is selected with [Bucket Throw](../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md).

## Compatibility

Layers define compatibility by limiting which groves can appear together. For example, an arid upper canopy should not usually be paired with lush jungle lower canopy, and a cultivated orchard canopy should usually avoid wild, high-density understory. Compatibility is authored at the forest layering level by choosing distributions that make sense together.

The layer stack also controls visual hierarchy:

* **Ground cover** paints the lowest terrain surface.
* **Tufts** add small upright texture above ground cover.
* **Understory** fills bushes, large grasses, and short woody vegetation.
* **Lower canopy** fills the subcanopy beneath larger trees.
* **Upper canopy** defines the dominant tree layer and skyline.

Each layer should be optional. A forest cell may have no upper canopy, no lower canopy, or no understory if the selected layering calls for open terrain.

## Shape

A forest layering is defined like this:

```rust
pub struct ExampleForestLayering {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (GroundCoverGroveA, 1.0),
            (GroundCoverGroveB, 2.0),
        ],
        flop: [
            (None, 4.0),
            (GroundCoverGroveC, 1.0),
        ],
    },
    tufts: TuftsLayer [
        (None, 2.0),
        (TuftsGroveA, 1.0),
        (TuftsGroveB, 1.0),
    ],
    understory: UnderstoryLayer [
        (None, 1.0),
        (UnderstoryGroveA, 1.0),
        (UnderstoryGroveB, 0.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 2.0),
        (LowerCanopyGroveA, 1.0),
        (LowerCanopyGroveB, 0.25),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 2.0),
        (UpperCanopyGroveA, 1.0),
        (UpperCanopyGroveB, 0.25),
    ],
}
```

## Evaluation

For each forest cell:

1. Use [Hopscotch](../02-selection/README.md#3521-hopscotch) to select a forest layering.
2. For each layer in that layering, use Bucket Throw to select a grove or `None`.
3. Instantiate the selected groves independently, using each grove's own cell size, density, offset, noise, and placement constraints.
4. Allow groves from different layers to overlap when their vertical role permits it.

The forest layer system should not try to deduplicate individual plants across layers. Avoiding bad overlap is the job of layer authoring, grove placement constraints, and later LOD or collision rules where needed.

Subsections:

* [3.5.3.1: Ground Cover Layer](./01-ground-cover-layer/README.md)
* [3.5.3.2: Tufts Layer](./02-tufts-layer/README.md)
* [3.5.3.3: Understory Layer](./03-understory-layer/README.md)
* [3.5.3.4: Lower Canopy Layer](./04-lower-canopy-layer/README.md)
* [3.5.3.5: Upper Canopy Layer](./05-upper-canopy-layer/README.md)
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
