# Spotting Intelligence

`spotting-intelligence` is the physics-independent half of Maybraid perception.
It defines:

- semantic [`InterestLayers`](src/layers.rs) for directive/subject matching;
- capsule visibility samples through [`SpotBounds`](src/bounds.rs);
- [`SpotSubject`](src/subject.rs), [`SpotDirective`](src/directive.rs), and
  [`SpottingUser`](src/user.rs) components and policy;
- remembered [`SpottedContact`](src/contact.rs) snapshots; and
- deterministic candidate ranking and sample-budget helpers.

The core crate depends only on Bevy. Install a backend such as
`spotting-intelligence-avian` to discover colliders and test line of sight.

```rust
use bevy::prelude::*;
use spotting_intelligence::{
	InterestLayers, SpotBounds, SpotDirective, SpotSubject, SpottingUser,
};

let subject = SpotSubject::new(
	InterestLayers::CHARACTER,
	SpotBounds::capsule(0.4, 0.9),
);
let observer = SpottingUser::new(
	Vec3::Y * 1.6,
	[SpotDirective::new(InterestLayers::CHARACTER, 40.0)],
);
```

Subjects used by the Avian backend also need a collider on
`lod_avian::PhysicsInteractionLayer::Animated`. Static occluders need the
`Fixed` layer.

Backend scheduling is application-owned. `SpottingSystems::Observe` contains
ordered `Prepare`, `Discover`, and `Refresh` subsets, allowing discovery to use a
slower timer while bounded retained-contact refresh runs each Update.
`SpottingObserveLimits` independently caps observers and allocated visibility
samples for both phases; see the [Avian backend](../avian/README.md) for setup,
service-round semantics, and the limits of contact freshness under overload.
