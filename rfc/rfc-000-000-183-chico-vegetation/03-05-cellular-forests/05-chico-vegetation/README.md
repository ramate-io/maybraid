# 3.5.5: Chico Vegetation

This page is subsection **3.5.5** of [RFC-183: Chico Vegetation](../../README.md)

Initial Hopscotch distribution sketch for Chico vegetation:

```rust
pub enum ChicoVegetationHopscotch {
    LushJungle(Bucket {
        weight: 2.0,
        adjacencies: [
            (TrapThicket, 1.5),
            (LiamsSummer, 1.0)
            (Kumulipo, 1.0),
            (OpenTropics, 0.5),
            (Riparian, 0.5),
            (DamasEdge, 0.1),
            (SunsBarren, 0.1)
            // Loop back somewhat common
            (LushJungle, 0.5)
        ],
        item: LushJungle
    }),
    Riparian(Bucket {
        weight: 4.0,
        adjacencies: [
            (Riparian, 0.5),
            (Storybook, 1.0),
            (Meadowland, 1.0),
            (MiRobles, 0.75),
            (FruitPlains, 0.75),
            (LushJungle, 0.5),
        ],
        item: Riparian
    }),
    Taiga(Bucket {
        weight: 1.0,
        adjacencies: [
            (Seceda, 1.25),
            (OldNevada, 0.75),
            (TemperateHoly, 0.5),
            (OldSteppe, 0.5),
            // Loopback fairly common
            (Taiga, 1.0)
        ],
        item: Taiga
    }),
    LiamsSummer(Bucket {
        weight: 1.0,
        adjacencies: [
            (WestMaui, 1.25),
            (OpenTropics, 1.0),
            (DamasEdge, 0.75),
            (Bush, 0.75),
            (Kumulipo, 0.5),
        ],
        item: LiamsSummer
    }),
    OwlsDesert(Bucket {
        weight: 2.0,
        adjacencies: [
            (SunsBarren, 1.0),
            (DamasEdge, 1.0),
            (OldNevada, 0.75),
            (Bush, 0.5),
            (SteppeDown, 0.5),
            // Loop back common
            (OwlsDesert, 2.0)
        ],
        item: OwlsDesert
    }),
    MiRobles(Bucket {
        weight: 2.0,
        adjacencies: [
            (UpperPark, 1.25),
            (Meadowland, 1.0),
            (Riparian, 0.5),
            (FruitPlains, 0.5),
            (SteppeDown, 0.5),
            (Bush, 0.5),
            // Loop back somewhat common
            (MiRobles, 1.0)
        ],
        item: MiRobles
    }),
    Seceda(Bucket {
        weight: 1.0,
        adjacencies: [
            (Taiga, 1.25),
            (OldNevada, 1.0),
            (SteppeDown, 0.75),
            (TemperateHoly, 0.5),
            (SunsBarren, 0.5)
        ],
        item: Seceda
    }),
    Kumulipo(Bucket {
        weight: 0.75,
        adjacencies: [
            (LushJungle, 1.0),
            (OpenTropics, 1.0),
            (WestMaui, 0.75),
            (LiamsSummer, 0.5),
            (DamasEdge, 0.5),
        ],
        item: Kumulipo
    }),
    Waiguo(Bucket {
        weight: 1.0,
        adjacencies: [
            (AgTown, 1.5),
            (FruitPlains, 1.25),
            (Storybook, 0.5),
            (Riparian, 0.5),
            (MiRobles, 0.5),
            // Loop back common
            (Waiguo, 1.0)
        ],
        item: Waiguo
    }),
    AgTown(Bucket {
        weight: 0.75,
        adjacencies: [
            (Waiguo, 0.25),
            (FruitPlains, 1.0),
            (Meadowland, 0.75),
            (SunsBarren, 0.25),
            // Loop back common
            (AgTown, 2.0)
        ],
        item: AgTown
    }),
    SunsBarren(Bucket {
        weight: 2.0,
        adjacencies: [
            (OwlsDesert, 1.0),
            (SteppeDown, 1.0),
            (OldSteppe, 0.75),
            (OldNevada, 0.5),
            (Meadowland, 0.25),
        ],
        item: SunsBarren
    }),
    TemperateHoly(Bucket {
        weight: 0.75,
        adjacencies: [
            (Riperian, 2.0)
            (Taiga, 0.75),
            (Seceda, 0.5),
            (Storybook, 0.5),
            (Meadowland, 0.5),
            (MiRobles, 0.5),
        ],
        item: TemperateHoly
    }),
    OldSteppe(Bucket {
        weight: 2.0,
        adjacencies: [
            (SteppeDown, 1.25),
            (Meadowland, 1.0),
            (SunsBarren, 0.75),
            (UpperPark, 0.75),
            (OldNevada, 0.5),
            (Kumulipo, 0.25).
            // Loop back common
            (OldSteppe, 2.0)
        ],
        item: OldSteppe
    }),
    TrapThicket(Bucket {
        weight: 0.75,
        adjacencies: [
            (LushJungle, 1.5),
            (OpenTropics, 0.75),
            (Kumulipo, 0.5),
            (Storybook, 0.25),
            // Loop back common
            (TrapThicket, 1.0)
        ],
        item: TrapThicket
    }),
    Bush(Bucket {
        weight: 2.0,
        adjacencies: [
            (UpperPark, 1.0),
            (SteppeDown, 1.0),
            (WestMaui, 0.75),
            (DamasEdge, 0.75),
            (MiRobles, 0.5),
            (OwlsDesert, 0.5),
            (SunsBarren, 0.5),
            // Looop back very common
            (Bush, 3.0)
        ],
        item: Bush
    }),
    OldNevada(Bucket {
        weight: 1.0,
        adjacencies: [
            (OwlsDesert, 1.0),
            (Seceda, 1.0),
            (Taiga, 0.75),
            (SunsBarren, 0.75),
            (SteppeDown, 0.75),
        ],
        item: OldNevada
    }),
    Storybook(Bucket {
        weight: 2.0,
        adjacencies: [
            (Riparian, 1.0),
            (Meadowland, 0.75),
            (TemperateHoly, 0.5),
            (Waiguo, 0.5),
            (LushJungle, 0.25),
            // Loop back common
            (Storybook, 1.0)
        ],
        item: Storybook
    }),
    Meadowland(Bucket {
        weight: 1.0,
        adjacencies: [
            (Meadowland, 0.5),
            (OldSteppe, 1.0),
            (MiRobles, 1.0),
            (Riparian, 0.75),
            (FruitPlains, 0.75),
            (UpperPark, 0.75),
            (Storybook, 0.5),
        ],
        item: Meadowland
    }),
    FruitPlains(Bucket {
        weight: 1.0,
        adjacencies: [
            (Waiguo, 0.5),
            (AgTown, 1.0),
            (Meadowland, 1.0),
            (MiRobles, 0.75),
            (Riparian, 0.5),
        ],
        item: FruitPlains
    }),
    DamasEdge(Bucket {
        weight: 0.75,
        adjacencies: [
            (OwlsDesert, 1.0),
            (Bush, 0.75),
            (LiamsSummer, 0.75),
            (OpenTropics, 0.5),
            (WestMaui, 0.5),
        ],
        item: DamasEdge
    }),
    OpenTropics(Bucket {
        weight: 1.25,
        adjacencies: [
            (WestMaui, 1.0),
            (LiamsSummer, 1.0),
            (Kumulipo, 0.75),
            (LushJungle, 1.5),
            (DamasEdge, 0.5),
            // Loop back common
            (OpenTropics, 1.0)
        ],
        item: OpenTropics
    }),
    WestMaui(Bucket {
        weight: 1.25,
        adjacencies: [
            (OpenTropics, 1.0),
            (LiamsSummer, 1.0),
            (Bush, 0.75),
            (DamasEdge, 0.5),
            (SteppeDown, 0.5),
        ],
        item: WestMaui
    }),
    UpperPark(Bucket {
        weight: 1.25,
        adjacencies: [
            (MiRobles, 1.0),
            (Bush, 1.0),
            (SteppeDown, 1.0),
            (OldSteppe, 0.75),
            (Meadowland, 0.75),
        ],
        item: UpperPark
    }),
    SteppeDown(Bucket {
        weight: 1.25,
        adjacencies: [
            (UpperPark, 1.0),
            (OldSteppe, 1.0),
            (Bush, 1.0),
            (SunsBarren, 0.75),
            (OldNevada, 0.5),
            (WestMaui, 0.5),
            // loop back common
            (SteppeDown, 2.0)
        ],
        item: SteppeDown
    }),
}
```

## Summary

This distribution is organized around a few coherent neighborhoods:

* **Wet tropical** layerings cluster around Lush Jungle, Trap Thicket, Kumulipo, Open Tropics, West Maui, and Liam's Summer.
* **Dry and open** layerings cluster around Owl's Desert, Sun's Barren, Steppe Down, Bush, Old Nevada, and Old Steppe.
* **Temperate and meadow** layerings cluster around Mi Robles, Upper Park, Meadowland, Storybook, Riparian, and Temperate Holy.
* **Cultivated** layerings cluster around Ag Town, Waiguo, and Fruit Plains, with bridges back into Meadowland, Riparian, and Mi Robles.
* Loop-backs are used on stable identities that should resist noisy drift: jungle, desert, oak, bush, steppe, storybook, cultivated, and open tropics.

## Connectivity

Node labels include bucket weights in parentheses. Edge labels show adjacency weights. The pseudocode above remains the source of truth.

```mermaid
graph TD
    LushJungle["Lush Jungle (2.0)"]
    Riparian["Riparian (4.0)"]
    Taiga["Taiga (1.0)"]
    LiamsSummer["Liam's Summer (1.0)"]
    OwlsDesert["Owl's Desert (2.0)"]
    MiRobles["Mi Robles (2.0)"]
    Seceda["Seceda (1.0)"]
    Kumulipo["Kumulipo (0.75)"]
    Waiguo["Waiguo (1.0)"]
    AgTown["Ag Town (0.75)"]
    SunsBarren["Sun's Barren (2.0)"]
    TemperateHoly["Temperate Holy (0.75)"]
    OldSteppe["Old Steppe (2.0)"]
    TrapThicket["Trap Thicket (0.75)"]
    Bush["Bush (2.0)"]
    OldNevada["Old Nevada (1.0)"]
    Storybook["Storybook (2.0)"]
    Meadowland["Meadowland (1.0)"]
    FruitPlains["Fruit Plains (1.0)"]
    DamasEdge["Damas Edge (0.75)"]
    OpenTropics["Open Tropics (1.25)"]
    WestMaui["West Maui (1.25)"]
    UpperPark["Upper Park (1.25)"]
    SteppeDown["Steppe Down (1.25)"]

    LushJungle -->|1.5| TrapThicket
    LushJungle -->|1.0| LiamsSummer
    LushJungle -->|1.0| Kumulipo
    LushJungle -->|0.5| OpenTropics
    LushJungle -->|0.5| Riparian
    LushJungle -->|0.1| DamasEdge
    LushJungle -->|0.1| SunsBarren
    LushJungle -->|0.5| LushJungle

    Riparian -->|0.5| Riparian
    Riparian -->|1.0| Storybook
    Riparian -->|1.0| Meadowland
    Riparian -->|0.75| MiRobles
    Riparian -->|0.75| FruitPlains
    Riparian -->|0.5| LushJungle

    Taiga -->|1.25| Seceda
    Taiga -->|0.75| OldNevada
    Taiga -->|0.5| TemperateHoly
    Taiga -->|0.5| OldSteppe
    Taiga -->|1.0| Taiga

    LiamsSummer -->|1.25| WestMaui
    LiamsSummer -->|1.0| OpenTropics
    LiamsSummer -->|0.75| DamasEdge
    LiamsSummer -->|0.75| Bush
    LiamsSummer -->|0.5| Kumulipo

    OwlsDesert -->|1.0| SunsBarren
    OwlsDesert -->|1.0| DamasEdge
    OwlsDesert -->|0.75| OldNevada
    OwlsDesert -->|0.5| Bush
    OwlsDesert -->|0.5| SteppeDown
    OwlsDesert -->|2.0| OwlsDesert

    MiRobles -->|1.25| UpperPark
    MiRobles -->|1.0| Meadowland
    MiRobles -->|0.5| Riparian
    MiRobles -->|0.5| FruitPlains
    MiRobles -->|0.5| SteppeDown
    MiRobles -->|0.5| Bush
    MiRobles -->|1.0| MiRobles

    Seceda -->|1.25| Taiga
    Seceda -->|1.0| OldNevada
    Seceda -->|0.75| SteppeDown
    Seceda -->|0.5| TemperateHoly
    Seceda -->|0.5| SunsBarren

    Kumulipo -->|1.0| LushJungle
    Kumulipo -->|1.0| OpenTropics
    Kumulipo -->|0.75| WestMaui
    Kumulipo -->|0.5| LiamsSummer
    Kumulipo -->|0.5| DamasEdge

    Waiguo -->|1.5| AgTown
    Waiguo -->|1.25| FruitPlains
    Waiguo -->|0.5| Storybook
    Waiguo -->|0.5| Riparian
    Waiguo -->|0.5| MiRobles
    Waiguo -->|1.0| Waiguo

    AgTown -->|0.25| Waiguo
    AgTown -->|1.0| FruitPlains
    AgTown -->|0.75| Meadowland
    AgTown -->|0.25| SunsBarren
    AgTown -->|2.0| AgTown

    SunsBarren -->|1.0| OwlsDesert
    SunsBarren -->|1.0| SteppeDown
    SunsBarren -->|0.75| OldSteppe
    SunsBarren -->|0.5| OldNevada
    SunsBarren -->|0.25| Meadowland

    TemperateHoly -->|2.0| Riparian
    TemperateHoly -->|0.75| Taiga
    TemperateHoly -->|0.5| Seceda
    TemperateHoly -->|0.5| Storybook
    TemperateHoly -->|0.5| Meadowland
    TemperateHoly -->|0.5| MiRobles

    OldSteppe -->|1.25| SteppeDown
    OldSteppe -->|1.0| Meadowland
    OldSteppe -->|0.75| SunsBarren
    OldSteppe -->|0.75| UpperPark
    OldSteppe -->|0.5| OldNevada
    OldSteppe -->|0.25| Kumulipo
    OldSteppe -->|2.0| OldSteppe

    TrapThicket -->|1.5| LushJungle
    TrapThicket -->|0.75| OpenTropics
    TrapThicket -->|0.5| Kumulipo
    TrapThicket -->|0.25| Storybook
    TrapThicket -->|1.0| TrapThicket

    Bush -->|1.0| UpperPark
    Bush -->|1.0| SteppeDown
    Bush -->|0.75| WestMaui
    Bush -->|0.75| DamasEdge
    Bush -->|0.5| MiRobles
    Bush -->|0.5| OwlsDesert
    Bush -->|0.5| SunsBarren
    Bush -->|3.0| Bush

    OldNevada -->|1.0| OwlsDesert
    OldNevada -->|1.0| Seceda
    OldNevada -->|0.75| Taiga
    OldNevada -->|0.75| SunsBarren
    OldNevada -->|0.75| SteppeDown

    Storybook -->|1.0| Riparian
    Storybook -->|0.75| Meadowland
    Storybook -->|0.5| TemperateHoly
    Storybook -->|0.5| Waiguo
    Storybook -->|0.25| LushJungle
    Storybook -->|1.0| Storybook

    Meadowland -->|0.5| Meadowland
    Meadowland -->|1.0| OldSteppe
    Meadowland -->|1.0| MiRobles
    Meadowland -->|0.75| Riparian
    Meadowland -->|0.75| FruitPlains
    Meadowland -->|0.75| UpperPark
    Meadowland -->|0.5| Storybook

    FruitPlains -->|0.5| Waiguo
    FruitPlains -->|1.0| AgTown
    FruitPlains -->|1.0| Meadowland
    FruitPlains -->|0.75| MiRobles
    FruitPlains -->|0.5| Riparian

    DamasEdge -->|1.0| OwlsDesert
    DamasEdge -->|0.75| Bush
    DamasEdge -->|0.75| LiamsSummer
    DamasEdge -->|0.5| OpenTropics
    DamasEdge -->|0.5| WestMaui

    OpenTropics -->|1.0| WestMaui
    OpenTropics -->|1.0| LiamsSummer
    OpenTropics -->|0.75| Kumulipo
    OpenTropics -->|1.5| LushJungle
    OpenTropics -->|0.5| DamasEdge
    OpenTropics -->|1.0| OpenTropics

    WestMaui -->|1.0| OpenTropics
    WestMaui -->|1.0| LiamsSummer
    WestMaui -->|0.75| Bush
    WestMaui -->|0.5| DamasEdge
    WestMaui -->|0.5| SteppeDown

    UpperPark -->|1.0| MiRobles
    UpperPark -->|1.0| Bush
    UpperPark -->|1.0| SteppeDown
    UpperPark -->|0.75| OldSteppe
    UpperPark -->|0.75| Meadowland

    SteppeDown -->|1.0| UpperPark
    SteppeDown -->|1.0| OldSteppe
    SteppeDown -->|1.0| Bush
    SteppeDown -->|0.75| SunsBarren
    SteppeDown -->|0.5| OldNevada
    SteppeDown -->|0.5| WestMaui
    SteppeDown -->|2.0| SteppeDown
```
