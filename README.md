# Maybraid
A game of procedural generation and peer-based state. 

> [!NOTE]
> Maybraid is currently in very early development. You will find mostly playgrounds from procedural generation and core mechanics concepts.

## General use

To use this repository, install [Determinate Systems Nix](https://determinate.systems/blog/determinate-nix-installer/). Then `cd` into the working directory for the repository and `nix develop`.

## Demos

### Navigable
Demos that you can navigate, but can't play as a character.

- [`demos/naturescapes`](/demos/naturescapes/): a demo allowing you to navigate the naturescapes of Maybraid. 

```shell
cargo run --release -p naturescapes-demo
```


### Playable
> [!NOTE]
> Nothing here yet. 

Demos where you can play as a character. 

## Playgrounds
There are several playgrounds in active development:

- [`playgrounds/skill-map`](/playgrounds/skill-map/): a playground demonstrating a very simple skill map which spawns fireballs when the user hits pink squares and locks down when the user hits blue squares. 

```shell
cargo run --release -p skill-map-playground
```

- [`playground/objects`](/playgrounds/objects/): a playground for inspecting various mesh objects. To run,

```shell
cargo run --release -p objects-playground
```

- [`playgrounds/terrain`](/playgrounds/): a playground for inspecting terrain and large-scale LOD concepts.

```shell
cargo run --release -p terrain-playground`
```
