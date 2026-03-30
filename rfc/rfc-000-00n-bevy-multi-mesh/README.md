# RFC-N: Bevy Multi-mesh

## Motivation

> [!NOTE]
> Below are some relevant references to this concept which preceded this proposal:
>
> - [#86](https://github.com/ramate-io/maybraid/issues/86)

In order to modularly compose mesh generation and animation systems, we will benefit from a multi-mesh system. This system should be responsible for inserting relevant `bevy::Transforms` or a similar object, s.t., we can assign higher order transformations and have them apply consistently down the multi-mesh structure.

This will eventually need to play nicely with animation systems. For example, an animation system may angle the legs on a character a particular way according to some kinematics. This higher-order multi-mesh system may then be used to produce the effect that the whole character multi-mesh should be moved by a certain amount following an explosion. Should the legs keep their angle? How can we make it easy for the implementer to decide? Can we wrap the multi-mesh behavior behind a trait? If we do, how do we still give some baseline behavior out of the box?

## Prior art

### In general

### Within `bevy`

#### `bevy` API
- **Entity hierarchy** (`ChildOf` / `Children`): child `Transform` is local to its parent; `GlobalTransform` is derived. Moving an ancestor moves the whole subtree—this is the usual way to compose “whole body” motion with “part-local” pose.
- **Scene and glTF loading**: imported scenes are often a tree of entities with meshes, materials, and transforms—multi-mesh assemblies are already a first-class import path.
- **`bevy_animation`**: clips target entities by paths; skinned meshes use joint hierarchies. Even without full skeletal pipelines, the same hierarchy idea applies to procedural parts.

#### `bevy` Community
- **Physics (Rapier, Avian, etc.)**: compound colliders and rigid-body children are typically parented, so sync rules match rendering—patterns for “one logical object, many pieces.”
- **Community examples**: tutorials and crates that build vehicles, doors, or characters as parented meshes rather than merging geometry—favor updating a few nodes over rebaking one mesh.

## Approaches Considered

- **Parented entities only**: rely on Bevy’s transform propagation; gameplay and animation write `Transform` at the right nodes. Minimal custom API; conventions document who owns which depth.
- **Trait / policy object**: abstract “apply command to assembly” so types can customize root vs part routing; more flexible, more API surface.
- **Flat meshes + sync systems**: keep siblings and copy offsets each frame—simple for legacy spawners, easier to get wrong under animation.

## Proposed Design

TBD as we narrow scope against this repo’s generators and command layer. Baseline expectation: align with **parented entities** unless a concrete need forces a heavier abstraction.
