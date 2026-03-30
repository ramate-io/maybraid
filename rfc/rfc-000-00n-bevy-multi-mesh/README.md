# RFC-N: Bevy Multi-mesh

## Motivation

> [!NOTE]
> Below are some relevant references to this concept which preceded this proposal:
>
> - [#86](https://github.com/ramate-io/maybraid/issues/86)

In order to modularly compose mesh generation and animation systems, we will benefit from a multi-mesh system. This system should be responsible for inserting relevant `bevy::Transforms` or a similar object, s.t., we can assign higher order transformations and have them apply consistently down the multi-mesh structure. 

This will eventually need to play nicely with animation systems. For example, an animation system may angle the legs on a character a particular way according to some kinematics. This higher-order multi-mesh system may then be used produce the effect that the whole character multi-mesh should be moved by a certain amount following an explosion. Should the legs keep their angle? How can we make it easy for the implementer to decide? Can we wrap the multi-mesh behavior behind a trait? If we do, how do we still give some baseline behavior out of the box?  

## Prior art

## Approaches Considered

## Proposed Design